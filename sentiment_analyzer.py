#!/usr/bin/env python3
"""
Sentiment Analysis Module for Sentiment Oracle

This module loads tweet data from JSON files, analyzes sentiment using a pre-trained
transformer model, and saves the results to new JSON files.
"""

import json
import os
import re
from datetime import datetime
from transformers import pipeline
from collections import Counter


def load_tweets(filename: str) -> list[dict]:
    """
    Load tweets from a JSON file.
    
    Args:
        filename: Path to the JSON file containing tweets
        
    Returns:
        List of tweet dictionaries
    """
    print(f"Loading tweets from {filename}...")
    
    with open(filename, 'r', encoding='utf-8') as f:
        tweets = json.load(f)
        
    print(f"Loaded {len(tweets)} tweets")
    return tweets


def adjust_financial_sentiment(text: str, label: str, score: float) -> tuple:
    """
    Adjust sentiment based on financial domain-specific terminology.
    
    Args:
        text: Tweet text
        label: Original sentiment label
        score: Original sentiment score
        
    Returns:
        Tuple of (adjusted_label, adjusted_score)
    """
    # Convert text to lowercase for easier matching
    text_lower = text.lower()
    
    # Financial positive terms
    positive_terms = [
        'bullish', 'moon', 'long term', 'support', 'strong', 'growth', 'pumping',
        'higher', 'upside', 'buy', 'buying', 'bought', 'momentum', 'target'
    ]
    
    # Financial negative terms
    negative_terms = [
        'bearish', 'crash', 'resistance', 'short term', 'down', 'dip', 'sell', 
        'selling', 'sold', 'overvalued', 'drag'
    ]
    
    # Check for conflicting signals
    positive_matches = sum(1 for term in positive_terms if term in text_lower)
    negative_matches = sum(1 for term in negative_terms if term in text_lower)
    
    # Strong financial signals override the general model
    if positive_matches > negative_matches and positive_matches >= 1:
        return "POSITIVE", max(score, 0.75)
    elif negative_matches > positive_matches and negative_matches >= 1:
        return "NEGATIVE", max(score, 0.75)
    elif 0.4 < score < 0.6:
        return "NEUTRAL", score
    else:
        return label, score


def analyze_sentiment(tweets: list[dict]) -> list[dict]:
    """
    Analyze sentiment of tweets using a pre-trained model.
    
    Args:
        tweets: List of tweet dictionaries
        
    Returns:
        List of dictionaries with sentiment analysis results
    """
    print("Initializing sentiment analysis model...")
    sentiment_analyzer = pipeline(
        "sentiment-analysis",
        model="distilbert-base-uncased-finetuned-sst-2-english"
    )
    
    results = []
    total_tweets = len(tweets)
    
    print(f"Analyzing sentiment for {total_tweets} tweets...")
    
    # Process tweets in batches to avoid memory issues
    batch_size = 8
    
    for i in range(0, total_tweets, batch_size):
        # Get the current batch
        batch = tweets[i:i+batch_size]
        
        # Extract text from each tweet
        texts = [tweet["content"] for tweet in batch]
        
        # Analyze sentiment
        sentiment_results = sentiment_analyzer(texts)
        
        # Combine original tweet data with sentiment results
        for j, sentiment in enumerate(sentiment_results):
            tweet = batch[j]
            
            # Get initial sentiment label and score
            label = sentiment["label"]
            score = sentiment["score"]
            
            # Apply domain-specific adjustments for financial terminology
            label, score = adjust_financial_sentiment(tweet["content"], label, score)
            
            result = {
                "id": tweet["id"],
                "text": tweet["content"],
                "label": label,
                "score": score,
                # Include additional metadata for tracing
                "date": tweet["date"],
                "username": tweet["username"],
                "source": tweet["source"]
            }
            
            results.append(result)
        
        # Print progress
        print(f"Processed {min(i+batch_size, total_tweets)}/{total_tweets} tweets")
    
    print(f"Sentiment analysis complete")
    return results


def save_results(results: list[dict], filename: str):
    """
    Save sentiment analysis results to a JSON file.
    
    Args:
        results: List of dictionaries with sentiment analysis results
        filename: Path to save the results
    """
    with open(filename, 'w', encoding='utf-8') as f:
        json.dump(results, f, ensure_ascii=False, indent=4)
    
    file_size = os.path.getsize(filename) / 1024  # Size in KB
    print(f"Saved {len(results)} results to {filename} ({file_size:.2f} KB)")


def print_sentiment_stats(results: list[dict]):
    """
    Print statistics about sentiment distribution.
    
    Args:
        results: List of dictionaries with sentiment analysis results
    """
    sentiment_counts = Counter([r["label"] for r in results])
    total = len(results)
    
    print("\nSentiment Distribution:")
    for label, count in sentiment_counts.items():
        percentage = (count / total) * 100
        bar_length = int(percentage / 2)
        bar = "#" * bar_length
        print(f"{label:<8} ({count:3d}, {percentage:5.1f}%): {bar}")


def main():
    """
    Main function to run sentiment analysis on both $SOL and Nifty 50 tweets.
    """
    print("Starting sentiment analysis process...")
    today = datetime.now().strftime('%Y-%m-%d')
    
    # Process $SOL tweets
    sol_input = f"SOL_{today}.json"
    sol_output = f"SOL_sentiment_{today}.json"
    
    sol_tweets = load_tweets(sol_input)
    sol_results = analyze_sentiment(sol_tweets)
    save_results(sol_results, sol_output)
    print_sentiment_stats(sol_results)
    
    # Process Nifty 50 tweets
    nifty_input = f"NIFTY_{today}.json"
    nifty_output = f"NIFTY_sentiment_{today}.json"
    
    nifty_tweets = load_tweets(nifty_input)
    nifty_results = analyze_sentiment(nifty_tweets)
    save_results(nifty_results, nifty_output)
    print_sentiment_stats(nifty_results)
    
    print("\nSentiment analysis completed successfully!")


if __name__ == "__main__":
    main() 