// Price Oracle Node - A decentralized price aggregation oracle for Solana
use clap::{Parser, Subcommand};
use log::{info, error};
use std::time::Duration;
use tokio::time::sleep;

mod data_sources;
mod consensus;
mod validator;
mod solana_client;
mod models;

use data_sources::{CoinGeckoSource, CoinMarketCapSource, BinanceSource, DataSource};
use consensus::ConsensusEngine;
use validator::PriceValidator;
use solana_client::SolanaOracleClient;
use models::ConsensusResult;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the oracle node
    Start {
        /// Asset to track (e.g., BTC, SOL, ETH)
        #[arg(short, long, default_value = "BTC")]
        asset: String,
        
        /// Update interval in seconds
        #[arg(short, long, default_value = "30")]
        interval: u64,
        
        /// Solana RPC URL
        #[arg(long, default_value = "https://api.devnet.solana.com")]
        rpc_url: String,
        
        /// Program ID for the oracle program
        #[arg(long)]
        program_id: Option<String>,
    },
    
    /// Run a single price update
    Update {
        /// Asset to update
        #[arg(short, long, default_value = "BTC")]
        asset: String,
        
        /// Program ID for the oracle program
        #[arg(long)]
        program_id: Option<String>,
    },
    
    /// Test data sources
    TestSources {
        /// Asset to test
        #[arg(short, long, default_value = "BTC")]
        asset: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Start { asset, interval, rpc_url, program_id } => {
            start_oracle_node(asset, interval, rpc_url, program_id).await?;
        },
        Commands::Update { asset, program_id } => {
            run_single_update(asset, program_id).await?;
        },
        Commands::TestSources { asset } => {
            test_data_sources(asset).await?;
        },
    }
    
    Ok(())
}

async fn start_oracle_node(
    asset: String,
    interval: u64,
    rpc_url: String,
    program_id: Option<String>,
) -> anyhow::Result<()> {
    info!("Starting Price Oracle Node for asset: {}", asset);
    
    // Initialize data sources
    let coin_gecko = CoinGeckoSource::new();
    let coin_market_cap = CoinMarketCapSource::new();
    let binance = BinanceSource::new();
    
    let data_sources: Vec<Box<dyn DataSource>> = vec![
        Box::new(coin_gecko),
        Box::new(coin_market_cap),
        Box::new(binance),
    ];
    
    // Initialize consensus engine
    let consensus_engine = ConsensusEngine::new();
    
    // Initialize price validator
    let mut validator = PriceValidator::new();
    
    // Initialize Solana client
    let solana_client = SolanaOracleClient::new(&rpc_url, program_id)?;
    
    info!("Oracle node initialized successfully");
    info!("Update interval: {} seconds", interval);
    info!("Oracle Public Key: {}", solana_client.get_oracle_pubkey());
    info!("Get SOL from faucet: https://faucet.solana.com/");
    
    // Main oracle loop
    loop {
        match run_price_update(&asset, &data_sources, &consensus_engine, &mut validator, &solana_client).await {
            Ok(result) => {
                info!("Price update successful: {} = ${:.2} (confidence: {:.2})", 
                      result.asset, result.price, result.confidence);
            },
            Err(e) => {
                error!("Price update failed: {}", e);
            }
        }
        
        sleep(Duration::from_secs(interval)).await;
    }
}

async fn run_single_update(asset: String, program_id: Option<String>) -> anyhow::Result<()> {
    info!("Running single price update for: {}", asset);
    
    // Initialize components
    let coin_gecko = CoinGeckoSource::new();
    let coin_market_cap = CoinMarketCapSource::new();
    let binance = BinanceSource::new();
    
    let data_sources: Vec<Box<dyn DataSource>> = vec![
        Box::new(coin_gecko),
        Box::new(coin_market_cap),
        Box::new(binance),
    ];
    
    let consensus_engine = ConsensusEngine::new();
    let mut validator = PriceValidator::new();
    let solana_client = SolanaOracleClient::new("https://api.devnet.solana.com", program_id)?;
    
    // Run update
    let result = run_price_update(&asset, &data_sources, &consensus_engine, &mut validator, &solana_client).await?;
    
    println!("Price Update Result:");
    println!("Asset: {}", result.asset);
    println!("Price: ${:.2}", result.price);
    println!("Confidence: {:.2}", result.confidence);
    println!("Sources: {:?}", result.sources);
    println!("Consensus Score: {:.2}", result.consensus_score);
    
    Ok(())
}

async fn test_data_sources(asset: String) -> anyhow::Result<()> {
    info!("Testing data sources for asset: {}", asset);
    
    let coin_gecko = CoinGeckoSource::new();
    let coin_market_cap = CoinMarketCapSource::new();
    let binance = BinanceSource::new();
    
    let sources: Vec<(&str, Box<dyn DataSource>)> = vec![
        ("CoinGecko", Box::new(coin_gecko)),
        ("CoinMarketCap", Box::new(coin_market_cap)),
        ("Binance", Box::new(binance)),
    ];
    
    for (name, source) in sources {
        match source.fetch_price(&asset).await {
            Ok(price_data) => {
                println!("{}: ${:.2} (confidence: {:.2})", 
                         name, price_data.price, price_data.confidence);
            },
            Err(e) => {
                println!("{}: Error - {}", name, e);
            }
        }
    }
    
    Ok(())
}

async fn run_price_update(
    asset: &str,
    data_sources: &[Box<dyn DataSource>],
    consensus_engine: &ConsensusEngine,
    validator: &mut PriceValidator,
    solana_client: &SolanaOracleClient,
) -> anyhow::Result<ConsensusResult> {
    info!("Fetching price data for {}", asset);
    
    // Fetch prices from all sources
    let mut price_data_vec = Vec::new();
    
    for source in data_sources {
        match source.fetch_price(asset).await {
            Ok(data) => {
                info!("Fetched price from {}: ${:.2}", data.source, data.price);
                price_data_vec.push(data);
            },
            Err(e) => {
                error!("Failed to fetch price from {}: {}", source.name(), e);
            }
        }
    }
    
    if price_data_vec.is_empty() {
        return Err(anyhow::anyhow!("No price data available from any source"));
    }
    
    // Validate prices
    let validated_prices = validator.validate_prices(&price_data_vec)?;
    
    // Run consensus
    let consensus_result = consensus_engine.run_consensus(&validated_prices)?;
    
    info!("Consensus reached: ${:.2} (confidence: {:.2})", 
          consensus_result.price, consensus_result.confidence);
    
    // Submit to Solana (if configured)
    if let Err(e) = solana_client.submit_price(&consensus_result).await {
        error!("Failed to submit to Solana: {}", e);
        // Don't fail the entire update if Solana submission fails
    }
    
    Ok(consensus_result)
}
