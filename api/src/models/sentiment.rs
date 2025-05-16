use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Raw sentiment data as stored on-chain or in local files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentimentData {
    pub id: String,
    pub text: String,
    pub label: String,
    pub score: f64,
    #[serde(with = "chrono::serde::ts_string_option", default)]
    pub date: Option<DateTime<Utc>>,
    pub username: String,
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_key: Option<String>,
}

/// Signed sentiment data from the oracle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedSentimentData {
    pub data: SentimentData,
    pub signature: String,
    pub public_key: String,
}

/// API response format for /latest endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatestSentimentResponse {
    pub asset: String,
    pub date: String,
    pub sentiment: String,
    pub confidence: f64,
    pub signature: String,
    pub signer: String,
}

/// Request for the /verify endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyRequest {
    pub payload: SentimentData,
    pub signature: String,
    pub signer: String,
}

/// Response for the /verify endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyResponse {
    pub valid: bool,
}

/// Response for the /history endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryResponse {
    pub asset: String,
    pub data: Vec<HistorySentimentEntry>,
}

/// Single sentiment entry for the history endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistorySentimentEntry {
    pub date: String,
    pub sentiment: String,
    pub confidence: f64,
} 