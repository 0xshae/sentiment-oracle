// Solana client for submitting price data to the blockchain
use anyhow::Result;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    signature::{Keypair, Signer},
    pubkey::Pubkey,
    transaction::Transaction,
    instruction::{AccountMeta, Instruction},
};
use std::str::FromStr;

use crate::models::ConsensusResult;
use price_oracle_program::{PriceOracleInstruction, get_account_size};

pub struct SolanaOracleClient {
    rpc_client: RpcClient,
    program_id: Option<Pubkey>,
    keypair: Keypair,
}

impl SolanaOracleClient {
    fn load_or_create_keypair() -> Result<Keypair> {
        // Use Solana CLI keypair
        let solana_config_path = std::env::var("SOLANA_CONFIG_FILE")
            .unwrap_or_else(|_| format!("{}/.config/solana/id.json", std::env::var("HOME").unwrap()));
        
        if std::path::Path::new(&solana_config_path).exists() {
            // Load Solana CLI keypair
            let keypair_data = std::fs::read_to_string(&solana_config_path)?;
            let keypair_bytes: Vec<u8> = serde_json::from_str(&keypair_data)?;
            let keypair = Keypair::from_bytes(&keypair_bytes)?;
            println!("🔑 Using Solana CLI keypair: {}", keypair.pubkey());
            Ok(keypair)
        } else {
            // Fallback to local keypair
            let keypair_path = "oracle_keypair.json";
            
            if std::path::Path::new(keypair_path).exists() {
                // Load existing keypair
                let keypair_data = std::fs::read_to_string(keypair_path)?;
                let keypair_bytes: Vec<u8> = serde_json::from_str(&keypair_data)?;
                Ok(Keypair::from_bytes(&keypair_bytes)?)
            } else {
                // Generate new keypair and save it
                let keypair = Keypair::new();
                let keypair_bytes = keypair.to_bytes();
                let keypair_vec: Vec<u8> = keypair_bytes.to_vec();
                std::fs::write(keypair_path, serde_json::to_string(&keypair_vec)?)?;
                println!("🔑 Generated new oracle keypair: {}", keypair.pubkey());
                println!("💾 Saved to: {}", keypair_path);
                Ok(keypair)
            }
        }
    }
    
    pub fn new(rpc_url: &str, program_id: Option<String>) -> Result<Self> {
        let rpc_client = RpcClient::new_with_commitment(
            rpc_url.to_string(),
            CommitmentConfig::confirmed(),
        );
        
        let program_id = if let Some(id_str) = program_id {
            Some(Pubkey::from_str(&id_str)?)
        } else {
            None
        };
        
        // Load or generate keypair for this oracle node
        let keypair = Self::load_or_create_keypair()?;
        
        Ok(Self {
            rpc_client,
            program_id,
            keypair,
        })
    }
    
    pub async fn submit_price(&self, consensus_result: &ConsensusResult) -> Result<()> {
        if self.program_id.is_none() {
            log::info!("No program ID configured, skipping Solana submission");
            return Ok(());
        }
        
        let program_id = self.program_id.unwrap();
        
        log::info!("Submitting price to Solana: {} = ${:.2}", 
                  consensus_result.asset, consensus_result.price);
        
        // REAL blockchain submission
        self.submit_to_blockchain(consensus_result, program_id).await?;
        
        Ok(())
    }
    
    async fn submit_to_blockchain(
        &self, 
        consensus_result: &ConsensusResult, 
        program_id: Pubkey
    ) -> Result<()> {
        log::info!("🚀 REAL BLOCKCHAIN SUBMISSION to Solana program: {}", program_id);
        
        // Check if we have SOL for transaction fees
        let balance = self.get_sol_balance().await?;
        if balance < 0.001 {
            log::warn!("⚠️  Low SOL balance: {:.6} SOL. Need at least 0.001 SOL for transaction fees", balance);
            log::info!("💡 Get SOL from: https://faucet.solana.com/");
            return Err(anyhow::anyhow!("Insufficient SOL balance for transaction"));
        }
        
        log::info!("💰 Oracle balance: {:.6} SOL", balance);
        
        // Create or get oracle account
        let oracle_account = self.get_oracle_account_address(&consensus_result.asset, program_id);
        log::info!("📍 Oracle account: {}", oracle_account);
        
        // Check if account exists
        match self.rpc_client.get_account(&oracle_account) {
            Ok(_) => {
                log::info!("✅ Oracle account exists");
            },
            Err(_) => {
                log::info!("🆕 Creating new oracle account...");
                self.create_oracle_account(&consensus_result.asset).await?;
            }
        }
        
        // Sign the price data with our oracle keypair
        let price_data = format!("{}{}{}{}", 
            consensus_result.asset, 
            consensus_result.price, 
            consensus_result.timestamp.timestamp(),
            consensus_result.confidence
        );
        
        let signature = self.keypair.sign_message(price_data.as_bytes());
        let signer_pubkey = self.keypair.pubkey().to_bytes();
        
        // Create the instruction data
        let instruction = PriceOracleInstruction::SubmitPrice {
            asset: consensus_result.asset.clone(),
            price: consensus_result.price,
            confidence: consensus_result.confidence,
            timestamp: consensus_result.timestamp.timestamp(),
            sources: consensus_result.sources.clone(),
            consensus_score: consensus_result.consensus_score,
            signature: signature.as_ref().to_vec(),
            signer: signer_pubkey,
        };
        
        // Serialize the instruction
        let instruction_data = borsh::to_vec(&instruction)?;
        
        // Create the instruction
        let submit_ix = Instruction {
            program_id,
            accounts: vec![
                AccountMeta::new(oracle_account, false),
                AccountMeta::new(self.keypair.pubkey(), true),
            ],
            data: instruction_data,
        };
        
        // Create and send transaction
        let recent_blockhash = self.rpc_client.get_latest_blockhash()?;
        let transaction = Transaction::new_signed_with_payer(
            &[submit_ix],
            Some(&self.keypair.pubkey()),
            &[&self.keypair],
            recent_blockhash,
        );
        
        log::info!("📤 Submitting transaction...");
        let signature = self.rpc_client.send_and_confirm_transaction(&transaction)?;
        
        log::info!("🎉 SUCCESS! Transaction submitted: {}", signature);
        log::info!("🔗 View on Solana Explorer: https://explorer.solana.com/tx/{}", signature);
        log::info!("📊 Price data: {} = ${:.2} (confidence: {:.2})", 
                  consensus_result.asset, consensus_result.price, consensus_result.confidence);
        
        Ok(())
    }
    
    fn get_oracle_account_address(&self, asset: &str, program_id: Pubkey) -> Pubkey {
        // Generate deterministic account address based on asset and oracle pubkey
        let oracle_pubkey = self.keypair.pubkey();
        let seed = format!("oracle_{}", asset);
        Pubkey::create_with_seed(&oracle_pubkey, &seed, &program_id).unwrap()
    }
    
    pub fn get_oracle_pubkey(&self) -> Pubkey {
        self.keypair.pubkey()
    }
    
    pub fn get_program_id(&self) -> Option<Pubkey> {
        self.program_id
    }
    
    pub async fn create_oracle_account(&self, asset: &str) -> Result<Pubkey> {
        if self.program_id.is_none() {
            return Err(anyhow::anyhow!("No program ID configured"));
        }
        
        let program_id = self.program_id.unwrap();
        
        // Calculate required account size
        let sources = vec!["CoinGecko".to_string(), "CoinMarketCap".to_string(), "Binance".to_string()];
        let account_size = get_account_size(asset, &sources);
        
        // Get rent exemption
        let rent = self.rpc_client.get_minimum_balance_for_rent_exemption(account_size)?;
        
        // Generate deterministic account address
        let oracle_account = self.get_oracle_account_address(asset, program_id);
        
        // Create account instruction using create_with_seed
        let create_account_ix = solana_sdk::system_instruction::create_account_with_seed(
            &self.keypair.pubkey(),
            &oracle_account,
            &self.keypair.pubkey(),
            &format!("oracle_{}", asset),
            rent,
            account_size as u64,
            &program_id,
        );
        
        // Initialize account instruction
        let init_ix = Instruction {
            program_id,
            accounts: vec![AccountMeta::new(oracle_account, false)],
            data: borsh::to_vec(&PriceOracleInstruction::InitializeAccount)?,
        };
        
        // Create and send transaction
        let recent_blockhash = self.rpc_client.get_latest_blockhash()?;
        let transaction = Transaction::new_signed_with_payer(
            &[create_account_ix, init_ix],
            Some(&self.keypair.pubkey()),
            &[&self.keypair],
            recent_blockhash,
        );
        
        let signature = self.rpc_client.send_and_confirm_transaction(&transaction)?;
        
        log::info!("✅ Created oracle account: {}", oracle_account);
        log::info!("🔗 Transaction signature: {}", signature);
        
        Ok(oracle_account)
    }
    
    pub async fn get_account_balance(&self, pubkey: &Pubkey) -> Result<u64> {
        let balance = self.rpc_client.get_balance(pubkey)?;
        Ok(balance)
    }
    
    pub async fn get_sol_balance(&self) -> Result<f64> {
        let balance = self.get_account_balance(&self.keypair.pubkey()).await?;
        Ok(balance as f64 / 1_000_000_000.0) // Convert lamports to SOL
    }
}

// Helper trait removed - using borsh::to_vec directly
