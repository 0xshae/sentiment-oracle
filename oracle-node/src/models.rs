// Data models for the price oracle
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// Price data from a single source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceData {
    pub asset: String,
    pub price: f64,
    pub confidence: f64,
    pub timestamp: DateTime<Utc>,
    pub source: String,
    pub volume_24h: Option<f64>,
    pub market_cap: Option<f64>,
}

/// Consensus result from multiple sources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusResult {
    pub asset: String,
    pub price: f64,
    pub confidence: f64,
    pub timestamp: DateTime<Utc>,
    pub sources: Vec<String>,
    pub consensus_score: f64,
    pub price_variance: f64,
    pub outlier_count: usize,
}

/// Oracle configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OracleConfig {
    pub asset: String,
    pub update_interval: u64,
    pub rpc_url: String,
    pub program_id: Option<String>,
    pub min_confidence: f64,
    pub max_price_variance: f64,
}

/// Data source reliability score
#[derive(Debug, Clone)]
pub struct SourceReliability {
    pub source_name: String,
    pub reliability_score: f64,
    pub success_rate: f64,
    pub avg_response_time: f64,
    pub last_update: DateTime<Utc>,
}

/// Consensus parameters
#[derive(Debug, Clone)]
pub struct ConsensusParams {
    pub min_sources: usize,
    pub max_outlier_percentage: f64,
    pub confidence_threshold: f64,
    pub price_variance_threshold: f64,
}

impl Default for ConsensusParams {
    fn default() -> Self {
        Self {
            min_sources: 2,
            max_outlier_percentage: 0.3,
            confidence_threshold: 0.7,
            price_variance_threshold: 0.05, // 5% variance threshold
        }
    }
}

/// Price validation result
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub reason: Option<String>,
    pub adjusted_price: Option<f64>,
    pub confidence_adjustment: f64,
}

impl PriceData {
    pub fn new(asset: String, price: f64, source: String) -> Self {
        Self {
            asset,
            price,
            confidence: 0.8, // Default confidence
            timestamp: Utc::now(),
            source,
            volume_24h: None,
            market_cap: None,
        }
    }
    
    pub fn with_confidence(mut self, confidence: f64) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }
    
    pub fn with_volume(mut self, volume: f64) -> Self {
        self.volume_24h = Some(volume);
        self
    }
    
    pub fn with_market_cap(mut self, market_cap: f64) -> Self {
        self.market_cap = Some(market_cap);
        self
    }
}

impl ConsensusResult {
    pub fn new(asset: String, price: f64, sources: Vec<String>) -> Self {
        Self {
            asset,
            price,
            confidence: 0.8,
            timestamp: Utc::now(),
            sources,
            consensus_score: 0.8,
            price_variance: 0.0,
            outlier_count: 0,
        }
    }
    
    pub fn with_confidence(mut self, confidence: f64) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }
    
    pub fn with_consensus_score(mut self, score: f64) -> Self {
        self.consensus_score = score.clamp(0.0, 1.0);
        self
    }
    
    pub fn with_variance(mut self, variance: f64) -> Self {
        self.price_variance = variance;
        self
    }
    
    pub fn with_outliers(mut self, count: usize) -> Self {
        self.outlier_count = count;
        self
    }
}
