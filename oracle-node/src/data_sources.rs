// Data sources for fetching price data
use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde_json::Value;
use std::time::Duration;

use crate::models::PriceData;

#[async_trait]
pub trait DataSource: Send + Sync {
    async fn fetch_price(&self, asset: &str) -> Result<PriceData>;
    fn name(&self) -> &str;
    fn base_url(&self) -> &str;
}

/// CoinGecko API data source
pub struct CoinGeckoSource {
    client: Client,
    base_url: String,
}

impl CoinGeckoSource {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .expect("Failed to create HTTP client");
        
        Self {
            client,
            base_url: "https://api.coingecko.com/api/v3".to_string(),
        }
    }
    
    fn get_coin_id(&self, asset: &str) -> String {
        match asset.to_uppercase().as_str() {
            "BTC" => "bitcoin".to_string(),
            "ETH" => "ethereum".to_string(),
            "SOL" => "solana".to_string(),
            "ADA" => "cardano".to_string(),
            "DOT" => "polkadot".to_string(),
            "MATIC" => "matic-network".to_string(),
            "AVAX" => "avalanche-2".to_string(),
            "LINK" => "chainlink".to_string(),
            "UNI" => "uniswap".to_string(),
            "AAVE" => "aave".to_string(),
            _ => asset.to_lowercase(),
        }
    }
}

#[async_trait]
impl DataSource for CoinGeckoSource {
    async fn fetch_price(&self, asset: &str) -> Result<PriceData> {
        let coin_id = self.get_coin_id(asset);
        let url = format!("{}/simple/price?ids={}&vs_currencies=usd&include_24hr_vol=true&include_market_cap=true", 
                         self.base_url, coin_id);
        
        let response = self.client.get(&url).send().await?;
        
        if !response.status().is_success() {
            return Err(anyhow::anyhow!("CoinGecko API error: {}", response.status()));
        }
        
        let json: Value = response.json().await?;
        
        if let Some(coin_data) = json.get(coin_id) {
            let price = coin_data["usd"].as_f64()
                .ok_or_else(|| anyhow::anyhow!("Invalid price data"))?;
            
            let volume_24h = coin_data["usd_24h_vol"].as_f64();
            let market_cap = coin_data["usd_market_cap"].as_f64();
            
            Ok(PriceData::new(asset.to_string(), price, "CoinGecko".to_string())
                .with_confidence(0.9) // CoinGecko is highly reliable
                .with_volume(volume_24h.unwrap_or(0.0))
                .with_market_cap(market_cap.unwrap_or(0.0)))
        } else {
            Err(anyhow::anyhow!("Asset {} not found", asset))
        }
    }
    
    fn name(&self) -> &str {
        "CoinGecko"
    }
    
    fn base_url(&self) -> &str {
        &self.base_url
    }
}

/// CoinMarketCap API data source
pub struct CoinMarketCapSource {
    client: Client,
    base_url: String,
}

impl CoinMarketCapSource {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .expect("Failed to create HTTP client");
        
        Self {
            client,
            base_url: "https://pro-api.coinmarketcap.com/v1".to_string(),
        }
    }
    
    fn get_symbol(&self, asset: &str) -> String {
        match asset.to_uppercase().as_str() {
            "BTC" => "BTC".to_string(),
            "ETH" => "ETH".to_string(),
            "SOL" => "SOL".to_string(),
            "ADA" => "ADA".to_string(),
            "DOT" => "DOT".to_string(),
            "MATIC" => "MATIC".to_string(),
            "AVAX" => "AVAX".to_string(),
            "LINK" => "LINK".to_string(),
            "UNI" => "UNI".to_string(),
            "AAVE" => "AAVE".to_string(),
            _ => asset.to_string(),
        }
    }
}

#[async_trait]
impl DataSource for CoinMarketCapSource {
    async fn fetch_price(&self, asset: &str) -> Result<PriceData> {
        // Note: CoinMarketCap requires an API key in production
        // For demo purposes, we'll simulate the response
        let symbol = self.get_symbol(asset);
        
        // Simulate CoinMarketCap response (in production, you'd use real API)
        let simulated_price = match symbol.as_str() {
            "BTC" => 45230.50,
            "ETH" => 2650.75,
            "SOL" => 98.45,
            "ADA" => 0.45,
            "DOT" => 6.78,
            "MATIC" => 0.89,
            "AVAX" => 25.67,
            "LINK" => 12.34,
            "UNI" => 5.67,
            "AAVE" => 89.12,
            _ => 100.0, // Default price
        };
        
        // Add some random variation to simulate real data
        let variation = (rand::random::<f64>() - 0.5) * 0.02; // ±1% variation
        let price = simulated_price * (1.0 + variation);
        
        Ok(PriceData::new(asset.to_string(), price, "CoinMarketCap".to_string())
            .with_confidence(0.85) // CoinMarketCap is reliable
            .with_volume(1000000.0) // Simulated volume
            .with_market_cap(price * 1000000.0)) // Simulated market cap
    }
    
    fn name(&self) -> &str {
        "CoinMarketCap"
    }
    
    fn base_url(&self) -> &str {
        &self.base_url
    }
}

/// Binance API data source
pub struct BinanceSource {
    client: Client,
    base_url: String,
}

impl BinanceSource {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .expect("Failed to create HTTP client");
        
        Self {
            client,
            base_url: "https://api.binance.com/api/v3".to_string(),
        }
    }
    
    fn get_symbol(&self, asset: &str) -> String {
        format!("{}USDT", asset.to_uppercase())
    }
}

#[async_trait]
impl DataSource for BinanceSource {
    async fn fetch_price(&self, asset: &str) -> Result<PriceData> {
        let symbol = self.get_symbol(asset);
        let url = format!("{}/ticker/price?symbol={}", self.base_url, symbol);
        
        let response = self.client.get(&url).send().await?;
        
        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Binance API error: {}", response.status()));
        }
        
        let json: Value = response.json().await?;
        
        let price_str = json["price"].as_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid price data"))?;
        
        let price = price_str.parse::<f64>()?;
        
        Ok(PriceData::new(asset.to_string(), price, "Binance".to_string())
            .with_confidence(0.95) // Binance is very reliable for spot prices
            .with_volume(2000000.0) // Simulated volume
            .with_market_cap(price * 2000000.0)) // Simulated market cap
    }
    
    fn name(&self) -> &str {
        "Binance"
    }
    
    fn base_url(&self) -> &str {
        &self.base_url
    }
}
