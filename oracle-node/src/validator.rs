// Price validation and quality assessment
use anyhow::Result;
use std::collections::HashMap;

use crate::models::{PriceData, ValidationResult};

pub struct PriceValidator {
    // Historical price data for validation
    price_history: HashMap<String, Vec<f64>>,
    max_history_size: usize,
}

impl PriceValidator {
    pub fn new() -> Self {
        Self {
            price_history: HashMap::new(),
            max_history_size: 100,
        }
    }
    
    pub fn validate_prices(&mut self, price_data: &[PriceData]) -> Result<Vec<PriceData>> {
        let mut validated_prices = Vec::new();
        
        for data in price_data {
            match self.validate_single_price(data) {
                Ok(validation) => {
                    if validation.is_valid {
                        let mut validated_data = data.clone();
                        
                        // Apply any price adjustments
                        if let Some(adjusted_price) = validation.adjusted_price {
                            validated_data.price = adjusted_price;
                        }
                        
                        // Apply confidence adjustments
                        validated_data.confidence *= validation.confidence_adjustment;
                        validated_data.confidence = validated_data.confidence.clamp(0.0, 1.0);
                        
                        // Update price history before moving
                        self.update_price_history(&validated_data);
                        
                        validated_prices.push(validated_data);
                    } else {
                        log::warn!("Price validation failed for {} from {}: {:?}", 
                                  data.asset, data.source, validation.reason);
                    }
                },
                Err(e) => {
                    log::error!("Price validation error for {} from {}: {}", 
                               data.asset, data.source, e);
                }
            }
        }
        
        if validated_prices.is_empty() {
            return Err(anyhow::anyhow!("No valid prices found"));
        }
        
        Ok(validated_prices)
    }
    
    fn validate_single_price(&self, price_data: &PriceData) -> Result<ValidationResult> {
        // Basic price validation
        if price_data.price <= 0.0 {
            return Ok(ValidationResult {
                is_valid: false,
                reason: Some("Price must be positive".to_string()),
                adjusted_price: None,
                confidence_adjustment: 0.0,
            });
        }
        
        if price_data.price > 1_000_000.0 {
            return Ok(ValidationResult {
                is_valid: false,
                reason: Some("Price too high (possible error)".to_string()),
                adjusted_price: None,
                confidence_adjustment: 0.0,
            });
        }
        
        // Check against historical data if available
        if let Some(history) = self.price_history.get(&price_data.asset) {
            if let Some(validation) = self.validate_against_history(price_data, history) {
                return Ok(validation);
            }
        }
        
        // Check confidence bounds
        if price_data.confidence < 0.1 {
            return Ok(ValidationResult {
                is_valid: false,
                reason: Some("Confidence too low".to_string()),
                adjusted_price: None,
                confidence_adjustment: 0.0,
            });
        }
        
        // All validations passed
        Ok(ValidationResult {
            is_valid: true,
            reason: None,
            adjusted_price: None,
            confidence_adjustment: 1.0,
        })
    }
    
    fn validate_against_history(&self, price_data: &PriceData, history: &[f64]) -> Option<ValidationResult> {
        if history.len() < 3 {
            return None; // Not enough history
        }
        
        // Calculate historical statistics
        let mean = history.iter().sum::<f64>() / history.len() as f64;
        let variance = history.iter()
            .map(|x| (x - mean).powi(2))
            .sum::<f64>() / history.len() as f64;
        let std_dev = variance.sqrt();
        
        // Check for extreme price movements (> 3 standard deviations)
        let price_diff = (price_data.price - mean).abs();
        if price_diff > 3.0 * std_dev {
            // This could be a legitimate price movement or an error
            // We'll flag it but still accept it with reduced confidence
            return Some(ValidationResult {
                is_valid: true,
                reason: Some(format!("Large price movement detected: {:.2}%", 
                                    (price_diff / mean) * 100.0)),
                adjusted_price: None,
                confidence_adjustment: 0.7, // Reduce confidence for extreme movements
            });
        }
        
        // Check for suspiciously small movements (< 0.1% when history shows volatility)
        if std_dev > mean * 0.01 && price_diff < mean * 0.001 {
            return Some(ValidationResult {
                is_valid: true,
                reason: Some("Suspiciously small price movement".to_string()),
                adjusted_price: None,
                confidence_adjustment: 0.8,
            });
        }
        
        None // No issues found
    }
    
    fn update_price_history(&mut self, price_data: &PriceData) {
        let history = self.price_history.entry(price_data.asset.clone()).or_insert_with(Vec::new);
        
        history.push(price_data.price);
        
        // Keep only recent history
        if history.len() > self.max_history_size {
            history.remove(0);
        }
    }
    
    pub fn get_price_statistics(&self, asset: &str) -> Option<PriceStatistics> {
        self.price_history.get(asset).map(|history| {
            if history.is_empty() {
                return PriceStatistics::default();
            }
            
            let mean = history.iter().sum::<f64>() / history.len() as f64;
            let variance = history.iter()
                .map(|x| (x - mean).powi(2))
                .sum::<f64>() / history.len() as f64;
            let std_dev = variance.sqrt();
            
            let min = history.iter().fold(f64::INFINITY, |a, &b| a.min(b));
            let max = history.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
            
            PriceStatistics {
                count: history.len(),
                mean,
                std_dev,
                min,
                max,
                variance,
            }
        })
    }
}

#[derive(Debug, Clone)]
pub struct PriceStatistics {
    pub count: usize,
    pub mean: f64,
    pub std_dev: f64,
    pub min: f64,
    pub max: f64,
    pub variance: f64,
}

impl Default for PriceStatistics {
    fn default() -> Self {
        Self {
            count: 0,
            mean: 0.0,
            std_dev: 0.0,
            min: 0.0,
            max: 0.0,
            variance: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_validate_positive_price() {
        let mut validator = PriceValidator::new();
        
        let price_data = vec![
            PriceData::new("BTC".to_string(), 45000.0, "Test".to_string()),
        ];
        
        let result = validator.validate_prices(&price_data).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].price, 45000.0);
    }
    
    #[test]
    fn test_validate_negative_price() {
        let mut validator = PriceValidator::new();
        
        let price_data = vec![
            PriceData::new("BTC".to_string(), -100.0, "Test".to_string()),
        ];
        
        let result = validator.validate_prices(&price_data);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_validate_zero_price() {
        let mut validator = PriceValidator::new();
        
        let price_data = vec![
            PriceData::new("BTC".to_string(), 0.0, "Test".to_string()),
        ];
        
        let result = validator.validate_prices(&price_data);
        assert!(result.is_err());
    }
}
