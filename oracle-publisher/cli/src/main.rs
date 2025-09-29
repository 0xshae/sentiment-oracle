// Price Oracle CLI - A tool to sign and submit price data to Solana
use clap::{Parser, Subcommand};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    signature::{read_keypair_file, Keypair, Signer},
    pubkey::Pubkey,
    system_instruction::create_account,
    transaction::Transaction,
    instruction::{AccountMeta, Instruction},
};
use solana_cli_config::Config;
use std::fs::File;
use std::io::Read;
use std::str::FromStr;
use serde::{Serialize, Deserialize};
use sha2::{Sha256, Digest};
use ed25519_dalek::{Keypair as DalekKeypair, Signer as DalekSigner};
use rand::rngs::OsRng;
use borsh::BorshSerialize;
use price_oracle_program::{
    PriceOracleInstruction,
    get_account_size,
};

// Define the price payload structure
#[derive(Serialize, Deserialize, Debug, Clone)]
struct PriceData {
    asset: String,
    price: f64,
    confidence: f64,
    timestamp: i64,
    sources: Vec<String>,
    consensus_score: f64,
}

// Define the structure for signed data
#[derive(Serialize, Deserialize, Debug)]
struct SignedSentimentData {
    data: SentimentData,
    signature: Vec<u8>,
    signer: Vec<u8>,
}

// Define the CLI arguments
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// The Solana RPC URL
    #[arg(short, long, default_value = "https://api.devnet.solana.com")]
    url: String,
    
    /// The keypair file to use for signing
    #[arg(short, long)]
    keypair: Option<String>,
    
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Generate a new keypair
    GenerateKeypair {
        /// Output file
        #[arg(short, long)]
        output: String,
    },
    
    /// Sign sentiment data
    Sign {
        /// Input JSON file containing sentiment data
        #[arg(short, long)]
        input: String,
        
        /// Output file for the signed data
        #[arg(short, long)]
        output: String,
    },
    
    /// Create a new account to store sentiment data
    CreateAccount {
        /// The tweet ID to estimate account size
        #[arg(short, long)]
        tweet_id: String,
        
        /// The text content to estimate account size
        #[arg(short, long)]
        text: String,
        
        /// The username to estimate account size
        #[arg(short, long)]
        username: String,
        
        /// The date to estimate account size
        #[arg(short, long)]
        date: String,
        
        /// The source to estimate account size
        #[arg(short, long)]
        source: String,
    },
    
    /// Submit signed sentiment data to Solana
    Submit {
        /// Input file containing the signed sentiment data
        #[arg(short, long)]
        input: String,
        
        /// The Solana program ID
        #[arg(short, long)]
        program_id: String,
        
        /// The account to store the sentiment data
        #[arg(short, long)]
        account: String,
    },
}

fn main() {
    let cli = Cli::parse();
    
    // Get the RPC client
    let rpc_client = RpcClient::new_with_commitment(cli.url, CommitmentConfig::confirmed());
    
    // Get the keypair from the file or config
    let keypair = match cli.keypair {
        Some(keypair_path) => read_keypair_file(&keypair_path).expect("Failed to read keypair"),
        None => {
            let config = Config::load(&Config::default_config_file_path()).expect("Failed to load Solana CLI config");
            read_keypair_file(&config.keypair_path).expect("Failed to read keypair from config")
        }
    };
    
    match cli.command {
        Commands::GenerateKeypair { output } => {
            // Generate a new keypair
            let mut rng = OsRng{};
            let dalek_keypair = DalekKeypair::generate(&mut rng);
            
            // Save the keypair to a file
            let keypair_bytes = dalek_keypair.to_bytes().to_vec();
            std::fs::write(&output, keypair_bytes).expect("Failed to write keypair to file");
            
            println!("Generated new keypair and saved to {}", output);
            println!("Public key: {}", hex::encode(dalek_keypair.public.to_bytes()));
        },
        Commands::Sign { input, output } => {
            // Read the sentiment data from the input file
            let mut file = File::open(&input).expect("Failed to open input file");
            let mut contents = String::new();
            file.read_to_string(&mut contents).expect("Failed to read input file");
            
            let sentiment_data: SentimentData = serde_json::from_str(&contents)
                .expect("Failed to parse sentiment data");
            
            // Canonicalize the JSON
            let canonical_json = serde_json::to_string(&sentiment_data)
                .expect("Failed to serialize sentiment data");
            
            // Hash the canonical JSON using SHA-256
            let mut hasher = Sha256::new();
            hasher.update(canonical_json.as_bytes());
            let hash = hasher.finalize();
            
            // Load ED25519 keypair for signing
            let mut rng = OsRng{};
            let dalek_keypair = DalekKeypair::generate(&mut rng); // In real-world, load from file
            
            // Sign the hash
            let signature = dalek_keypair.sign(&hash);
            
            // Create the signed data structure
            let signed_data = SignedSentimentData {
                data: sentiment_data,
                signature: signature.to_bytes().to_vec(),
                signer: dalek_keypair.public.to_bytes().to_vec(),
            };
            
            // Write the signed data to the output file
            let signed_json = serde_json::to_string_pretty(&signed_data)
                .expect("Failed to serialize signed data");
            std::fs::write(&output, signed_json).expect("Failed to write signed data to file");
            
            println!("Signed sentiment data and saved to {}", output);
            println!("Signature: {}", hex::encode(signature.to_bytes()));
            println!("Signer: {}", hex::encode(dalek_keypair.public.to_bytes()));
        },
        Commands::CreateAccount { tweet_id, text, username, date, source } => {
            // Calculate the required account size
            let account_size = get_account_size(&tweet_id, &text, &username, &date, &source);
            
            // Generate a new keypair for the account
            let account_keypair = Keypair::new();
            
            // Calculate the rent exemption
            let rent = rpc_client.get_minimum_balance_for_rent_exemption(account_size)
                .expect("Failed to get rent exemption");
            
            // Create the account
            let create_account_ix = create_account(
                &keypair.pubkey(),
                &account_keypair.pubkey(),
                rent,
                account_size as u64,
                &Pubkey::from_str("11111111111111111111111111111111").unwrap(), // Program ID placeholder
            );
            
            // Build and send the transaction
            let blockhash = rpc_client.get_latest_blockhash()
                .expect("Failed to get blockhash");
            let transaction = Transaction::new_signed_with_payer(
                &[create_account_ix],
                Some(&keypair.pubkey()),
                &[&keypair, &account_keypair],
                blockhash,
            );
            
            let signature = rpc_client.send_and_confirm_transaction(&transaction)
                .expect("Failed to send transaction");
            
            println!("Created account: {}", account_keypair.pubkey());
            println!("Transaction signature: {}", signature);
        },
        Commands::Submit { input, program_id, account } => {
            // Parse the program ID
            let program_id = Pubkey::from_str(&program_id)
                .expect("Invalid program ID");
            
            // Parse the account
            let account_pubkey = Pubkey::from_str(&account)
                .expect("Invalid account");
            
            // Read the signed sentiment data from the input file
            let mut file = File::open(&input).expect("Failed to open input file");
            let mut contents = String::new();
            file.read_to_string(&mut contents).expect("Failed to read input file");
            
            let signed_data: SignedSentimentData = serde_json::from_str(&contents)
                .expect("Failed to parse signed data");
            
            // Convert the signer from bytes to a Pubkey
            let mut signer_bytes = [0u8; 32];
            signer_bytes.copy_from_slice(&signed_data.signer);
            
            // Create the instruction to submit the sentiment data
            let submit_ix = SentimentInstruction::SubmitSentiment {
                tweet_id: signed_data.data.id,
                text: signed_data.data.text,
                label: signed_data.data.label,
                score: signed_data.data.score,
                date: signed_data.data.date,
                username: signed_data.data.username,
                source: signed_data.data.source,
                signature: signed_data.signature,
                signer: signer_bytes,
            };
            
            // Serialize the instruction
            let mut instruction_data = Vec::new();
            submit_ix.serialize(&mut instruction_data)
                .expect("Failed to serialize instruction");
            
            // Create the Solana instruction
            let accounts = vec![
                AccountMeta::new(account_pubkey, false),
                AccountMeta::new_readonly(keypair.pubkey(), true),
            ];
            
            let instruction = Instruction {
                program_id,
                accounts,
                data: instruction_data,
            };
            
            // Build and send the transaction
            let blockhash = rpc_client.get_latest_blockhash()
                .expect("Failed to get blockhash");
            let transaction = Transaction::new_signed_with_payer(
                &[instruction],
                Some(&keypair.pubkey()),
                &[&keypair],
                blockhash,
            );
            
            let signature = rpc_client.send_and_confirm_transaction(&transaction)
                .expect("Failed to send transaction");
            
            println!("Submitted sentiment data to Solana");
            println!("Transaction signature: {}", signature);
        },
    }
}

// Helper function to parse a Pubkey from a string
fn pubkey_from_str(s: &str) -> Pubkey {
    Pubkey::from_str(s).expect("Invalid pubkey")
} 