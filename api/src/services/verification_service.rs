use ed25519_dalek::{PublicKey, Signature};
use sha2::{Digest, Sha256};
use serde_json;
use base64::{Engine as _, engine::general_purpose};

use crate::models::{ApiError, SentimentData, VerifyRequest};

/// Service for verifying signatures on sentiment data
pub struct VerificationService;

impl VerificationService {
    /// Create a new instance of the verification service
    pub fn new() -> Self {
        Self {}
    }

    /// Verify a signature against the data and signer
    pub async fn verify(&self, request: VerifyRequest) -> Result<bool, ApiError> {
        let data_hash = self.hash_sentiment_data(&request.payload)?;
        let signature_bytes = self.decode_base64(&request.signature)?;
        let public_key_bytes = self.decode_base64(&request.signer)?;
        
        self.verify_signature(&data_hash, &signature_bytes, &public_key_bytes)
            .map_err(|e| {
                ApiError::SignatureVerificationFailed
            })
    }
    
    /// Hash the sentiment data using SHA-256
    fn hash_sentiment_data(&self, sentiment_data: &SentimentData) -> Result<Vec<u8>, ApiError> {
        let canonical_json = serde_json::to_string(sentiment_data)
            .map_err(|e| ApiError::BadRequest(format!("Failed to serialize data: {}", e)))?;
        
        let mut hasher = Sha256::new();
        hasher.update(canonical_json.as_bytes());
        let hash = hasher.finalize();
        
        Ok(hash.to_vec())
    }
    
    /// Decode base64 string to bytes
    fn decode_base64(&self, encoded: &str) -> Result<Vec<u8>, ApiError> {
        general_purpose::STANDARD.decode(encoded)
            .map_err(|e| ApiError::BadRequest(format!("Invalid base64 encoding: {}", e)))
    }
    
    /// Verify the signature using ED25519
    fn verify_signature(&self, data_hash: &[u8], signature_bytes: &[u8], public_key_bytes: &[u8]) -> Result<bool, ApiError> {
        // Convert bytes to ED25519 types
        let signature = Signature::from_bytes(signature_bytes)
            .map_err(|e| ApiError::BadRequest(format!("Invalid signature format: {}", e)))?;
        
        let public_key = PublicKey::from_bytes(public_key_bytes)
            .map_err(|e| ApiError::BadRequest(format!("Invalid public key format: {}", e)))?;
        
        // Verify the signature
        match public_key.verify_strict(data_hash, &signature) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
} 