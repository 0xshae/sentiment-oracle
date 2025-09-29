use std::collections::HashMap;
use std::env;
use std::fs;
use std::sync::{Arc, Mutex};
use std::io::Cursor;

use actix_cors::Cors;
use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder, middleware::Logger, ResponseError};
use anyhow::Result;
use base64::{Engine as _, engine::general_purpose};
use chrono::Utc;
use ed25519_dalek::{PublicKey, Signature};
use log::info;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use dotenv;

// ==== Models ====

/// Raw sentiment data as stored on-chain or in local files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentimentData {
    pub id: String,
    pub text: String,
    pub label: String,
    pub score: f64,
    // Using a custom date format for now to avoid the chrono::serde issue
    #[serde(default)]
    pub date: Option<String>,
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

/// Error type for API operations
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("Not found: {0}")]
    NotFound(String),
    
    #[error("Bad request: {0}")]
    BadRequest(String),
    
    #[error("Signature verification failed")]
    SignatureVerificationFailed,
    
    #[error("Internal server error: {0}")]
    InternalServerError(String),
}

impl ResponseError for ApiError {
    fn error_response(&self) -> HttpResponse {
        match self {
            ApiError::NotFound(_) => HttpResponse::NotFound().json(self.to_string()),
            ApiError::BadRequest(_) => HttpResponse::BadRequest().json(self.to_string()),
            ApiError::SignatureVerificationFailed => HttpResponse::BadRequest().json(self.to_string()),
            ApiError::InternalServerError(_) => HttpResponse::InternalServerError().json(self.to_string()),
        }
    }
}

/// Asset query parameter
#[derive(Deserialize)]
pub struct AssetQuery {
    pub asset: String,
}

// ==== Services ====

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
            Err(_) => {
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
            Err(_) => {
                Err(ApiError::NotFound(format!("No sentiment history found for {}", asset)))
            }
        }
    }

    /// Transform signed sentiment data to API response format
    fn transform_to_response(&self, asset: &str, data: SignedSentimentData) -> Result<LatestSentimentResponse, ApiError> {
        // Format the date string
        let date_str = data.data.date
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
        
        info!("Loading sentiment data from file: {}", file_path);
        let file_content = fs::read_to_string(&file_path)?;
        
        // Parse the JSON file
        let signed_data: serde_json::Value = serde_json::from_str(&file_content)?;
        
        // Create a SentimentData object from the parsed JSON
        let sentiment_data = SentimentData {
            id: "sample_0_1747301807".to_string(),
            text: "Sample sentiment data for $SOL".to_string(),
            label: signed_data["data"]["overall_sentiment"].as_str().unwrap_or("NEUTRAL").to_string(),
            score: signed_data["data"]["confidence"].as_f64().unwrap_or(0.5),
            date: Some(signed_data["data"]["date"].as_str().unwrap_or("2025-05-15").to_string()),
            username: "oracle".to_string(),
            source: "Sentiment Oracle".to_string(),
            signature: None,
            public_key: None,
        };
        
        // Create a SignedSentimentData object
        let signed_sentiment_data = SignedSentimentData {
            data: sentiment_data,
            signature: signed_data["signature"].as_str().unwrap_or("").to_string(),
            public_key: signed_data["public_key"].as_str().unwrap_or("").to_string(),
        };
        
        Ok(signed_sentiment_data)
    }
}

/// Service for verifying signatures on sentiment data
#[derive(Clone)]
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
            .map_err(|_| {
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

// ==== Handlers ====

/// Get the latest sentiment for an asset
#[get("/latest")]
async fn get_latest_sentiment(
    query: web::Query<AssetQuery>,
    sentiment_service: web::Data<SentimentService>,
) -> impl Responder {
    let asset = &query.asset;
    info!("GET /latest - asset: {}", asset);
    
    match sentiment_service.get_latest_sentiment(asset).await {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(e) => e.error_response(),
    }
}

/// Get sentiment history for an asset
#[get("/history")]
async fn get_sentiment_history(
    query: web::Query<AssetQuery>,
    sentiment_service: web::Data<SentimentService>,
) -> impl Responder {
    let asset = &query.asset;
    info!("GET /history - asset: {}", asset);
    
    match sentiment_service.get_sentiment_history(asset).await {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(e) => e.error_response(),
    }
}

/// Verify a signature on sentiment data
#[post("/verify")]
async fn verify_signature(
    req: web::Json<VerifyRequest>,
    verification_service: web::Data<VerificationService>,
) -> impl Responder {
    info!("POST /verify");
    
    match verification_service.verify(req.into_inner()).await {
        Ok(valid) => {
            HttpResponse::Ok().json(VerifyResponse { valid })
        },
        Err(e) => e.error_response(),
    }
}

/// Serve a simple HTML dashboard
#[get("/dashboard")]
async fn dashboard() -> impl Responder {
    info!("GET /dashboard");
    
    let html = r#"
    <!DOCTYPE html>
    <html>
    <head>
        <meta charset="UTF-8">
        <meta name="viewport" content="width=device-width, initial-scale=1.0">
        <title>Sentiment Oracle Dashboard</title>
        <style>
            body {
                font-family: Arial, sans-serif;
                margin: 0;
                padding: 20px;
                background-color: #f5f5f5;
            }
            h1, h2 {
                color: #333;
            }
            .container {
                max-width: 1200px;
                margin: 0 auto;
                background-color: white;
                padding: 20px;
                border-radius: 5px;
                box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
            }
            .sentiment-card {
                background-color: #fff;
                border-radius: 8px;
                padding: 20px;
                margin-bottom: 20px;
                box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
            }
            .sentiment-positive {
                border-left: 5px solid #4CAF50;
            }
            .sentiment-negative {
                border-left: 5px solid #F44336;
            }
            .sentiment-neutral {
                border-left: 5px solid #2196F3;
            }
            .label {
                display: inline-block;
                padding: 4px 8px;
                border-radius: 4px;
                color: white;
                font-weight: bold;
            }
            .label-positive {
                background-color: #4CAF50;
            }
            .label-negative {
                background-color: #F44336;
            }
            .label-neutral {
                background-color: #2196F3;
            }
            .confidence {
                display: inline-block;
                margin-left: 10px;
                color: #666;
            }
            .meta {
                margin-top: 10px;
                color: #666;
                font-size: 0.9em;
            }
            .footer {
                margin-top: 20px;
                text-align: center;
                color: #666;
                font-size: 0.8em;
            }
            #history-container {
                margin-top: 30px;
            }
            #refresh-button {
                background-color: #4CAF50;
                color: white;
                border: none;
                padding: 10px 15px;
                border-radius: 4px;
                cursor: pointer;
                font-size: 16px;
            }
            #refresh-button:hover {
                background-color: #45a049;
            }
        </style>
    </head>
    <body>
        <div class="container">
            <h1>Sentiment Oracle Dashboard</h1>
            
            <button id="refresh-button" onclick="loadData()">Refresh Data</button>
            
            <div id="latest-sentiment"></div>
            
            <div id="history-container">
                <h2>Sentiment History</h2>
                <div id="sentiment-history"></div>
            </div>
            
            <div class="footer">
                <p>Sentiment Oracle - Real-time sentiment analysis for financial markets</p>
            </div>
        </div>

        <script>
            // Set the default asset to track
            const asset = '$SOL';
            
            // Function to load the latest sentiment data
            async function loadLatestSentiment() {
                try {
                    const response = await fetch(`/latest?asset=${asset}`);
                    const data = await response.json();
                    
                    let sentimentClass = '';
                    let labelClass = '';
                    
                    if (data.sentiment === 'POSITIVE') {
                        sentimentClass = 'sentiment-positive';
                        labelClass = 'label-positive';
                    } else if (data.sentiment === 'NEGATIVE') {
                        sentimentClass = 'sentiment-negative';
                        labelClass = 'label-negative';
                    } else {
                        sentimentClass = 'sentiment-neutral';
                        labelClass = 'label-neutral';
                    }
                    
                    const confidencePct = (data.confidence * 100).toFixed(2);
                    
                    const html = `
                        <h2>Latest Sentiment for ${data.asset}</h2>
                        <div class="sentiment-card ${sentimentClass}">
                            <div>
                                <span class="label ${labelClass}">${data.sentiment}</span>
                                <span class="confidence">Confidence: ${confidencePct}%</span>
                            </div>
                            <div class="meta">
                                <p>Date: ${data.date}</p>
                                <p>Oracle: ${data.signer.substring(0, 10)}...</p>
                            </div>
                        </div>
                    `;
                    
                    document.getElementById('latest-sentiment').innerHTML = html;
                } catch (error) {
                    console.error('Error loading latest sentiment:', error);
                    document.getElementById('latest-sentiment').innerHTML = `
                        <div class="sentiment-card">
                            <p>Error loading latest sentiment data.</p>
                        </div>
                    `;
                }
            }
            
            // Function to load sentiment history
            async function loadSentimentHistory() {
                try {
                    const response = await fetch(`/history?asset=${asset}`);
                    const data = await response.json();
                    
                    if (data.data.length === 0) {
                        document.getElementById('sentiment-history').innerHTML = `
                            <div class="sentiment-card">
                                <p>No historical data available.</p>
                            </div>
                        `;
                        return;
                    }
                    
                    let html = '';
                    
                    data.data.forEach(item => {
                        let sentimentClass = '';
                        let labelClass = '';
                        
                        if (item.sentiment === 'POSITIVE') {
                            sentimentClass = 'sentiment-positive';
                            labelClass = 'label-positive';
                        } else if (item.sentiment === 'NEGATIVE') {
                            sentimentClass = 'sentiment-negative';
                            labelClass = 'label-negative';
                        } else {
                            sentimentClass = 'sentiment-neutral';
                            labelClass = 'label-neutral';
                        }
                        
                        const confidencePct = (item.confidence * 100).toFixed(2);
                        
                        html += `
                            <div class="sentiment-card ${sentimentClass}">
                                <div>
                                    <span class="label ${labelClass}">${item.sentiment}</span>
                                    <span class="confidence">Confidence: ${confidencePct}%</span>
                                </div>
                                <div class="meta">
                                    <p>Date: ${item.date}</p>
                                </div>
                            </div>
                        `;
                    });
                    
                    document.getElementById('sentiment-history').innerHTML = html;
                } catch (error) {
                    console.error('Error loading sentiment history:', error);
                    document.getElementById('sentiment-history').innerHTML = `
                        <div class="sentiment-card">
                            <p>Error loading sentiment history data.</p>
                        </div>
                    `;
                }
            }
            
            // Function to load all data
            function loadData() {
                loadLatestSentiment();
                loadSentimentHistory();
            }
            
            // Load data when the page loads
            document.addEventListener('DOMContentLoaded', loadData);
        </script>
    </body>
    </html>
    "#;
    
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(html)
}

// ==== Main ====

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Initialize environment
    dotenv::dotenv().ok();
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));
    
    // Configure data directory - default to "../oracle-publisher" if not specified
    let data_dir = env::var("DATA_DIR").unwrap_or_else(|_| "../oracle-publisher".to_string());
    info!("Using data directory: {}", data_dir);
    
    // Create services
    let sentiment_service = SentimentService::new(&data_dir);
    let verification_service = VerificationService::new();
    
    // Start HTTP server
    let bind_address = env::var("BIND_ADDRESS").unwrap_or_else(|_| "127.0.0.1:8080".to_string());
    info!("Starting server at {}", bind_address);
    
    HttpServer::new(move || {
        // Configure CORS
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header()
            .max_age(3600);
        
        App::new()
            .wrap(Logger::default())
            .wrap(cors)
            .app_data(web::Data::new(sentiment_service.clone()))
            .app_data(web::Data::new(verification_service.clone()))
            .service(get_latest_sentiment)
            .service(get_sentiment_history)
            .service(verify_signature)
            .service(dashboard)
    })
    .bind(bind_address)?
    .run()
    .await
}
