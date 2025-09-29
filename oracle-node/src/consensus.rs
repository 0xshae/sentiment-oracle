// Consensus mechanism for price aggregation
use anyhow::Result;

use crate::models::{PriceData, ConsensusResult, ConsensusParams};

pub struct ConsensusEngine {
    params: ConsensusParams,
}

impl ConsensusEngine {
    pub fn new() -> Self {
        Self {
            params: ConsensusParams::default(),
        }
    }
    
    pub fn with_params(params: ConsensusParams) -> Self {
        Self { params }
    }
    
    pub fn run_consensus(&self, price_data: &[PriceData]) -> Result<ConsensusResult> {
        if price_data.is_empty() {
            return Err(anyhow::anyhow!("No price data provided"));
        }
        
        if price_data.len() < self.params.min_sources {
            return Err(anyhow::anyhow!(
                "Insufficient sources: {} (minimum: {})", 
                price_data.len(), 
                self.params.min_sources
            ));
        }
        
        // Extract prices and calculate statistics
        let prices: Vec<f64> = price_data.iter().map(|p| p.price).collect();
        let sources: Vec<String> = price_data.iter().map(|p| p.source.clone()).collect();
        
        // Calculate basic statistics
        let mean_price = self.calculate_mean(&prices);
        let variance = self.calculate_variance(&prices, mean_price);
        let std_dev = variance.sqrt();
        
        // Detect outliers using modified Z-score
        let outliers = self.detect_outliers(&prices, mean_price, std_dev);
        let outlier_count = outliers.len();
        
        // Check if too many outliers
        let outlier_percentage = outlier_count as f64 / prices.len() as f64;
        if outlier_percentage > self.params.max_outlier_percentage {
            return Err(anyhow::anyhow!(
                "Too many outliers: {:.1}% (max: {:.1}%)", 
                outlier_percentage * 100.0, 
                self.params.max_outlier_percentage * 100.0
            ));
        }
        
        // Calculate weighted average excluding outliers
        let consensus_price = self.calculate_weighted_average(price_data, &outliers);
        
        // Calculate confidence based on multiple factors
        let confidence = self.calculate_confidence(price_data, variance, outlier_count);
        
        // Calculate consensus score
        let consensus_score = self.calculate_consensus_score(price_data, variance, outlier_count);
        
        // Create consensus result
        let asset = price_data[0].asset.clone();
        let result = ConsensusResult::new(asset, consensus_price, sources)
            .with_confidence(confidence)
            .with_consensus_score(consensus_score)
            .with_variance(variance)
            .with_outliers(outlier_count);
        
        Ok(result)
    }
    
    fn calculate_mean(&self, prices: &[f64]) -> f64 {
        prices.iter().sum::<f64>() / prices.len() as f64
    }
    
    fn calculate_variance(&self, prices: &[f64], mean: f64) -> f64 {
        let sum_squared_diff: f64 = prices.iter()
            .map(|price| (price - mean).powi(2))
            .sum();
        sum_squared_diff / prices.len() as f64
    }
    
    fn detect_outliers(&self, prices: &[f64], mean: f64, std_dev: f64) -> Vec<usize> {
        let mut outliers = Vec::new();
        
        for (i, price) in prices.iter().enumerate() {
            let z_score = (price - mean).abs() / std_dev;
            // Consider outliers if Z-score > 2.5 (more conservative than 2.0)
            if z_score > 2.5 {
                outliers.push(i);
            }
        }
        
        outliers
    }
    
    fn calculate_weighted_average(&self, price_data: &[PriceData], outliers: &[usize]) -> f64 {
        let mut total_weight = 0.0;
        let mut weighted_sum = 0.0;
        
        for (i, data) in price_data.iter().enumerate() {
            if !outliers.contains(&i) {
                let weight = data.confidence;
                weighted_sum += data.price * weight;
                total_weight += weight;
            }
        }
        
        if total_weight > 0.0 {
            weighted_sum / total_weight
        } else {
            // Fallback to simple average if no weights
            price_data.iter()
                .enumerate()
                .filter(|(i, _)| !outliers.contains(i))
                .map(|(_, data)| data.price)
                .sum::<f64>() / (price_data.len() - outliers.len()) as f64
        }
    }
    
    fn calculate_confidence(&self, price_data: &[PriceData], variance: f64, outlier_count: usize) -> f64 {
        // Base confidence from source confidences
        let avg_source_confidence = price_data.iter()
            .map(|p| p.confidence)
            .sum::<f64>() / price_data.len() as f64;
        
        // Adjust for variance (lower variance = higher confidence)
        let variance_factor = (1.0 - (variance / 10000.0).min(1.0)).max(0.1);
        
        // Adjust for outliers (fewer outliers = higher confidence)
        let outlier_factor = 1.0 - (outlier_count as f64 / price_data.len() as f64);
        
        // Combine factors
        let confidence = avg_source_confidence * variance_factor * outlier_factor;
        confidence.clamp(0.0, 1.0)
    }
    
    fn calculate_consensus_score(&self, price_data: &[PriceData], variance: f64, outlier_count: usize) -> f64 {
        // Consensus score based on agreement between sources
        let source_count = price_data.len();
        let outlier_penalty = outlier_count as f64 / source_count as f64;
        let variance_penalty = (variance / 10000.0).min(1.0);
        
        let base_score = 1.0 - outlier_penalty - variance_penalty;
        base_score.clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    
    #[test]
    fn test_consensus_with_good_data() {
        let engine = ConsensusEngine::new();
        
        let price_data = vec![
            PriceData::new("BTC".to_string(), 45000.0, "Source1".to_string())
                .with_confidence(0.9),
            PriceData::new("BTC".to_string(), 45100.0, "Source2".to_string())
                .with_confidence(0.8),
            PriceData::new("BTC".to_string(), 44900.0, "Source3".to_string())
                .with_confidence(0.85),
        ];
        
        let result = engine.run_consensus(&price_data).unwrap();
        
        assert_eq!(result.asset, "BTC");
        assert!(result.price > 44000.0 && result.price < 46000.0);
        assert!(result.confidence > 0.7);
        assert_eq!(result.outlier_count, 0);
    }
    
    #[test]
    fn test_consensus_with_outlier() {
        let engine = ConsensusEngine::new();
        
        let price_data = vec![
            PriceData::new("BTC".to_string(), 45000.0, "Source1".to_string())
                .with_confidence(0.9),
            PriceData::new("BTC".to_string(), 45100.0, "Source2".to_string())
                .with_confidence(0.8),
            PriceData::new("BTC".to_string(), 50000.0, "Source3".to_string()) // Outlier
                .with_confidence(0.7),
        ];
        
        let result = engine.run_consensus(&price_data).unwrap();
        
        assert_eq!(result.asset, "BTC");
        assert!(result.price < 46000.0); // Should exclude outlier
        assert!(result.outlier_count > 0);
    }
    
    #[test]
    fn test_consensus_insufficient_sources() {
        let engine = ConsensusEngine::new();
        
        let price_data = vec![
            PriceData::new("BTC".to_string(), 45000.0, "Source1".to_string()),
        ];
        
        let result = engine.run_consensus(&price_data);
        assert!(result.is_err());
    }
}
