// Price Oracle Program - A Solana program to store aggregated price data on-chain
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint,
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
    borsh::try_from_slice_unchecked,
    program_pack::IsInitialized,
};
use borsh::{BorshDeserialize, BorshSerialize};
use std::mem::size_of;
use sha2::{Sha256, Digest};

// Declare the program's entrypoint
entrypoint!(process_instruction);

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct PricePayload {
    pub is_initialized: bool,         // Used to check if the account has been initialized
    pub asset: String,                // Asset symbol (e.g., "BTC", "SOL")
    pub price: f64,                   // Aggregated price
    pub confidence: f64,              // Confidence score (0.0 to 1.0)
    pub timestamp: i64,              // Unix timestamp
    pub sources: Vec<String>,         // Data sources used
    pub consensus_score: f64,         // Consensus score
    pub signature: Vec<u8>,           // Signature of the payload
    pub signer: [u8; 32],            // The public key of the signer
}

// Implement the IsInitialized trait for PricePayload
impl IsInitialized for PricePayload {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

// Define the errors that can occur in the program
#[derive(Debug, thiserror::Error)]
pub enum PriceOracleError {
    #[error("Account not initialized")]
    UninitializedAccount,
    
    #[error("Invalid signature")]
    InvalidSignature,
    
    #[error("Account already initialized")]
    AccountAlreadyInitialized,
    
    #[error("Invalid price data")]
    InvalidPriceData,
    
    #[error("Consensus failed")]
    ConsensusFailed,
}

// Map the custom error to ProgramError
impl From<PriceOracleError> for ProgramError {
    fn from(e: PriceOracleError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

// Main instruction processor function
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    // Deserialize instruction data
    let instruction = PriceOracleInstruction::try_from_slice(instruction_data)?;
    
    match instruction {
        PriceOracleInstruction::InitializeAccount => {
            process_initialize_account(program_id, accounts)
        },
        PriceOracleInstruction::SubmitPrice {
            asset,
            price,
            confidence,
            timestamp,
            sources,
            consensus_score,
            signature,
            signer,
        } => {
            process_submit_price(
                program_id,
                accounts,
                asset,
                price,
                confidence,
                timestamp,
                sources,
                consensus_score,
                signature,
                signer,
            )
        }
    }
}

// Program instruction enum
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum PriceOracleInstruction {
    /// Initialize a new account
    /// Accounts expected: [writable] The account to initialize
    InitializeAccount,
    
    /// Submit a new price payload
    /// Accounts expected: 
    /// 0. [writable] The account to store the price data
    /// 1. [signer] The account of the oracle submitting the data
    SubmitPrice {
        asset: String,
        price: f64,
        confidence: f64,
        timestamp: i64,
        sources: Vec<String>,
        consensus_score: f64,
        signature: Vec<u8>,
        signer: [u8; 32],
    },
}

// Process account initialization
fn process_initialize_account(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let account = next_account_info(account_info_iter)?;
    
    // Check if the account is owned by the program
    if account.owner != program_id {
        msg!("Account doesn't belong to this program");
        return Err(ProgramError::IncorrectProgramId);
    }
    
    // Check if the account is already initialized
    if account.data.borrow().len() > 0 {
        let price_payload = try_from_slice_unchecked::<PricePayload>(&account.data.borrow())?;
        if price_payload.is_initialized {
            msg!("Account is already initialized");
            return Err(PriceOracleError::AccountAlreadyInitialized.into());
        }
    }
    
    // Create a new empty price payload
    let price_payload = PricePayload {
        is_initialized: true,
        asset: String::new(),
        price: 0.0,
        confidence: 0.0,
        timestamp: 0,
        sources: Vec::new(),
        consensus_score: 0.0,
        signature: Vec::new(),
        signer: [0; 32],
    };
    
    // Serialize and store the price payload
    price_payload.serialize(&mut *account.data.borrow_mut())?;
    
    msg!("Account initialized successfully");
    Ok(())
}

// Process price submission
fn process_submit_price(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    asset: String,
    price: f64,
    confidence: f64,
    timestamp: i64,
    sources: Vec<String>,
    consensus_score: f64,
    signature: Vec<u8>,
    signer: [u8; 32],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let account = next_account_info(account_info_iter)?;
    let submitter = next_account_info(account_info_iter)?;
    
    // Check if the account is owned by the program
    if account.owner != program_id {
        msg!("Account doesn't belong to this program");
        return Err(ProgramError::IncorrectProgramId);
    }
    
    // Check if the account is initialized
    let mut price_payload = try_from_slice_unchecked::<PricePayload>(&account.data.borrow())?;
    if !price_payload.is_initialized {
        msg!("Account is not initialized");
        return Err(PriceOracleError::UninitializedAccount.into());
    }
    
    // Check if the submitter signed the transaction
    if !submitter.is_signer {
        msg!("Submitter did not sign the transaction");
        return Err(ProgramError::MissingRequiredSignature);
    }
    
    // Validate price data
    if price <= 0.0 {
        msg!("Invalid price: {}", price);
        return Err(PriceOracleError::InvalidPriceData.into());
    }
    
    if confidence < 0.0 || confidence > 1.0 {
        msg!("Invalid confidence: {}", confidence);
        return Err(PriceOracleError::InvalidPriceData.into());
    }
    
    // Verify the signature (in a real-world application, we would verify the signature here)
    // For this implementation, we'll just log a message and save the signature
    msg!("Signature verification would happen here in a production system");
    
    // Update the price payload
    price_payload.asset = asset;
    price_payload.price = price;
    price_payload.confidence = confidence;
    price_payload.timestamp = timestamp;
    price_payload.sources = sources;
    price_payload.consensus_score = consensus_score;
    price_payload.signature = signature;
    price_payload.signer = signer;
    
    // Serialize and store the updated price payload
    price_payload.serialize(&mut *account.data.borrow_mut())?;
    
    msg!("Price data submitted successfully");
    Ok(())
}

// Helper function to calculate required account size
pub fn get_account_size(asset: &str, sources: &[String]) -> usize {
    let payload = PricePayload {
        is_initialized: true,
        asset: asset.to_string(),
        price: 0.0,
        confidence: 0.0,
        timestamp: 0,
        sources: sources.to_vec(),
        consensus_score: 0.0,
        signature: Vec::new(),
        signer: [0; 32],
    };
    
    let mut data = Vec::new();
    payload.serialize(&mut data).unwrap();
    
    // Add buffer space for the signature and any additional data
    data.len() + 256
} 