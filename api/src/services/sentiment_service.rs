use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use chrono::{DateTime, NaiveDate, Utc};
use log::{debug, error, info};
use serde_json;

use crate::models::{ApiError, HistoryResponse, HistorySentimentEntry, LatestSentimentResponse, SignedSentimentData};

/// Service for retrieving sentiment data
#[derive(Clone)]
pub struct SentimentService {
    // In-memory cache of latest sentiment by asset
    cache: Arc<Mutex<HashMap<String, SignedSentimentData>>>,
    // Path to sentiment data directory
    data_path: String,
}

impl SentimentService {
    /// Create a new instance of the sentiment service
    pub fn new(data_path: &str) -> Self {
        Self {
            cache: Arc::new(Mutex::new(HashMap::new())),
            data_path: data_path.to_string(),
        }
    }

    /// Get the latest sentiment for the specified asset
    pub async fn get_latest_sentiment(&self, asset: &str) -> Result<LatestSentimentResponse, ApiError> {
        // Check cache first
        if let Some(data) = self.cache.lock().unwrap().get(asset) {
            return self.transform_to_response(asset, data.clone());
        }

        // If not in cache, try to load from file
        match self.load_from_file(asset) {
            Ok(data) => {
                // Cache the result
                self.cache.lock().unwrap().insert(asset.to_string(), data.clone());
                self.transform_to_response(asset, data)
            }
            Err(e) => {
                error!("Failed to load sentiment data for {}: {}", asset, e);
                Err(ApiError::NotFound(format!("No sentiment data found for {}", asset)))
            }
        }
    }

    /// Get sentiment history for the specified asset
    pub async fn get_sentiment_history(&self, asset: &str) -> Result<HistoryResponse, ApiError> {
        // In a real implementation, we would query historical data from Solana
        // For now, we'll just return the latest data as a single entry
        
        match self.load_from_file(asset) {
            Ok(data) => {
                let date_str = data.data.date
                    .map(|d| d.format("%Y-%m-%d").to_string())
                    .unwrap_or_else(|| Utc::now().format("%Y-%m-%d").to_string());
                
                let entry = HistorySentimentEntry {
                    date: date_str,
                    sentiment: data.data.label.clone(),
                    confidence: data.data.score,
                };
                
                Ok(HistoryResponse {
                    asset: asset.to_string(),
                    data: vec![entry],
                })
            }
            Err(e) => {
                error!("Failed to load history data for {}: {}", asset, e);
                Err(ApiError::NotFound(format!("No sentiment history found for {}", asset)))
            }
        }
    }

    /// Transform signed sentiment data to API response format
    fn transform_to_response(&self, asset: &str, data: SignedSentimentData) -> Result<LatestSentimentResponse, ApiError> {
        // Format the date string
        let date_str = data.data.date
            .map(|d| d.format("%Y-%m-%d").to_string())
            .unwrap_or_else(|| Utc::now().format("%Y-%m-%d").to_string());
        
        Ok(LatestSentimentResponse {
            asset: asset.to_string(),
            date: date_str,
            sentiment: data.data.label,
            confidence: data.data.score,
            signature: data.signature,
            signer: data.public_key,
        })
    }

    /// Load sentiment data from file
    fn load_from_file(&self, asset: &str) -> Result<SignedSentimentData, anyhow::Error> {
        // For demo purposes, we'll just use the signed_sentiment.json file
        // In a real implementation, this would query from Solana based on the asset
        
        // Assuming we have different files for different assets in production
        let file_path = if asset.to_uppercase() == "$SOL" {
            format!("{}/signed_sentiment.json", self.data_path)
        } else {
            return Err(anyhow::anyhow!("Asset not supported"));
        };
        
        let file_content = fs::read_to_string(&file_path)?;
        let data: SignedSentimentData = serde_json::from_str(&file_content)?;
        
        Ok(data)
    }
} 