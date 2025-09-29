use actix_web::{web, HttpResponse, Responder, get, post};
use log::info;

use crate::models::{SentimentData, VerifyRequest, VerifyResponse};
use crate::services::{SentimentService, VerificationService};

/// Get the latest sentiment for an asset
#[get("/latest")]
pub async fn get_latest_sentiment(
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
pub async fn get_sentiment_history(
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
pub async fn verify_signature(
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

/// Asset query parameter
#[derive(serde::Deserialize)]
pub struct AssetQuery {
    pub asset: String,
}

/// Register all handlers with the app
pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(get_latest_sentiment)
       .service(get_sentiment_history)
       .service(verify_signature);
}