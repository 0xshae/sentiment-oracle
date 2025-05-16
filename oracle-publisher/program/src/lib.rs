// Sentiment Oracle Program - A Solana program to store signed sentiment data on-chain
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
pub struct SentimentPayload {
    pub is_initialized: bool,         // Used to check if the account has been initialized
    pub tweet_id: String,             // ID of the tweet
    pub text: String,                 // Text content of the tweet
    pub label: String,                // Sentiment label (POSITIVE, NEGATIVE, NEUTRAL)
    pub score: f64,                   // Sentiment score
    pub date: String,                 // Timestamp of the tweet
    pub username: String,             // Username of the tweet author
    pub source: String,               // Source of the tweet data
    pub signature: Vec<u8>,           // Signature of the payload
    pub signer: [u8; 32],             // The public key of the signer
}

// Implement the IsInitialized trait for SentimentPayload
impl IsInitialized for SentimentPayload {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

// Define the errors that can occur in the program
#[derive(Debug, thiserror::Error)]
pub enum SentimentError {
    #[error("Account not initialized")]
    UninitializedAccount,
    
    #[error("Invalid signature")]
    InvalidSignature,
    
    #[error("Account already initialized")]
    AccountAlreadyInitialized,
}

// Map the custom error to ProgramError
impl From<SentimentError> for ProgramError {
    fn from(e: SentimentError) -> Self {
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
    let instruction = SentimentInstruction::try_from_slice(instruction_data)?;
    
    match instruction {
        SentimentInstruction::InitializeAccount => {
            process_initialize_account(program_id, accounts)
        },
        SentimentInstruction::SubmitSentiment {
            tweet_id,
            text,
            label,
            score,
            date,
            username,
            source,
            signature,
            signer,
        } => {
            process_submit_sentiment(
                program_id,
                accounts,
                tweet_id,
                text,
                label,
                score,
                date,
                username,
                source,
                signature,
                signer,
            )
        }
    }
}

// Program instruction enum
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum SentimentInstruction {
    /// Initialize a new account
    /// Accounts expected: [writable] The account to initialize
    InitializeAccount,
    
    /// Submit a new sentiment payload
    /// Accounts expected: 
    /// 0. [writable] The account to store the sentiment data
    /// 1. [signer] The account of the oracle submitting the data
    SubmitSentiment {
        tweet_id: String,
        text: String,
        label: String,
        score: f64,
        date: String,
        username: String,
        source: String,
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
        let sentiment_payload = try_from_slice_unchecked::<SentimentPayload>(&account.data.borrow())?;
        if sentiment_payload.is_initialized {
            msg!("Account is already initialized");
            return Err(SentimentError::AccountAlreadyInitialized.into());
        }
    }
    
    // Create a new empty sentiment payload
    let sentiment_payload = SentimentPayload {
        is_initialized: true,
        tweet_id: String::new(),
        text: String::new(),
        label: String::new(),
        score: 0.0,
        date: String::new(),
        username: String::new(),
        source: String::new(),
        signature: Vec::new(),
        signer: [0; 32],
    };
    
    // Serialize and store the sentiment payload
    sentiment_payload.serialize(&mut *account.data.borrow_mut())?;
    
    msg!("Account initialized successfully");
    Ok(())
}

// Process sentiment submission
fn process_submit_sentiment(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    tweet_id: String,
    text: String,
    label: String,
    score: f64,
    date: String,
    username: String,
    source: String,
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
    let mut sentiment_payload = try_from_slice_unchecked::<SentimentPayload>(&account.data.borrow())?;
    if !sentiment_payload.is_initialized {
        msg!("Account is not initialized");
        return Err(SentimentError::UninitializedAccount.into());
    }
    
    // Check if the submitter signed the transaction
    if !submitter.is_signer {
        msg!("Submitter did not sign the transaction");
        return Err(ProgramError::MissingRequiredSignature);
    }
    
    // Verify the signature (in a real-world application, we would verify the signature here)
    // For this implementation, we'll just log a message and save the signature
    msg!("Signature verification would happen here in a production system");
    
    // Update the sentiment payload
    sentiment_payload.tweet_id = tweet_id;
    sentiment_payload.text = text;
    sentiment_payload.label = label;
    sentiment_payload.score = score;
    sentiment_payload.date = date;
    sentiment_payload.username = username;
    sentiment_payload.source = source;
    sentiment_payload.signature = signature;
    sentiment_payload.signer = signer;
    
    // Serialize and store the updated sentiment payload
    sentiment_payload.serialize(&mut *account.data.borrow_mut())?;
    
    msg!("Sentiment data submitted successfully");
    Ok(())
}

// Helper function to calculate required account size
pub fn get_account_size(tweet_id: &str, text: &str, username: &str, date: &str, source: &str) -> usize {
    let payload = SentimentPayload {
        is_initialized: true,
        tweet_id: tweet_id.to_string(),
        text: text.to_string(),
        label: String::from("POSITIVE"), // Placeholder
        score: 0.0,
        date: date.to_string(),
        username: username.to_string(),
        source: source.to_string(),
        signature: Vec::new(),
        signer: [0; 32],
    };
    
    let mut data = Vec::new();
    payload.serialize(&mut data).unwrap();
    
    // Add buffer space for the signature and any additional data
    data.len() + 256
} 