#!/usr/bin/env python3
"""
Twitter Scraper Module for Sentiment Oracle

This script scrapes tweets related to $SOL (Solana) and Nifty 50 index
using direct web scraping and saves them as structured JSON files for 
later sentiment analysis.
"""

import json
import os
import time
import random
from datetime import datetime
import requests
from bs4 import BeautifulSoup
import re
from urllib.parse import quote


def get_tweets(query, limit=100):
    """
    Scrape tweets from Twitter web interface using BeautifulSoup.
    
    Args:
        query: The search query to look for (e.g., "$SOL lang:en")
        limit: Maximum number of tweets to scrape (default: 100)
        
    Returns:
        A list of dictionaries containing tweet data
    """
    tweets = []
    encoded_query = quote(query)
    print(f"Scraping tweets for {query}...")
    
    # Different user agents to avoid detection
    user_agents = [
        'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36',
        'Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/92.0.4515.107 Safari/537.36',
        'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/90.0.4430.212 Safari/537.36',
        'Mozilla/5.0 (iPhone; CPU iPhone OS 12_2 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Mobile/15E148',
        'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/90.0.4430.93 Safari/537.36'
    ]
    
    headers = {
        'User-Agent': random.choice(user_agents),
        'Accept-Language': 'en-US,en;q=0.9',
        'Accept': 'text/html,application/xhtml+xml,application/xml',
        'Referer': 'https://twitter.com/',
        'Accept-Encoding': 'gzip, deflate, br'
    }
    
    # Try scraping from archive sites and financial sites that display tweets
    # Approach 1: Try Nitter instances (Twitter alternative frontends)
    nitter_instances = [
        f"https://nitter.net/search?f=tweets&q={encoded_query}&since=&until=&near=",
        f"https://nitter.unixfox.eu/search?f=tweets&q={encoded_query}",
        f"https://nitter.42l.fr/search?f=tweets&q={encoded_query}"
    ]
    
    # Approach 2: Try financial sites that aggregate tweets
    finance_sites = [
        f"https://stocktwits.com/symbol/{query.replace('$', '')}",
        f"https://tradingview.com/symbols/{query.replace('$', '')}/ideas/"
    ]
    
    # Try multiple sources
    sources = nitter_instances + finance_sites
    
    for source in sources:
        try:
            # Add randomized delay to avoid detection
            time.sleep(random.uniform(1.0, 3.0))
            
            response = requests.get(source, headers=headers, timeout=10)
            
            if response.status_code == 200:
                soup = BeautifulSoup(response.text, 'html.parser')
                
                # Different parsers for different sources
                if "nitter" in source:
                    tweet_elements = soup.select(".timeline-item")
                    for i, tweet_el in enumerate(tweet_elements):
                        if i >= limit:
                            break
                            
                        try:
                            # Extract tweet data from Nitter
                            content_el = tweet_el.select_one(".tweet-content")
                            tweet_link = tweet_el.select_one(".tweet-link")
                            username_el = tweet_el.select_one(".username")
                            date_el = tweet_el.select_one(".tweet-date")
                            stats = tweet_el.select(".tweet-stats .icon-container")
                            
                            if not all([content_el, username_el, date_el]):
                                continue
                                
                            content = content_el.get_text().strip()
                            username = username_el.get_text().strip()
                            date_str = date_el.get("title") if date_el.get("title") else date_el.get_text().strip()
                            tweet_id = tweet_link.get("href").split("/")[-1] if tweet_link else f"nitter_{i}"
                            
                            # Extract retweet and like counts
                            retweets = 0
                            likes = 0
                            if len(stats) >= 2:
                                retweets_text = stats[0].get_text().strip()
                                likes_text = stats[1].get_text().strip()
                                retweets = int(re.sub(r'\D', '', retweets_text)) if re.sub(r'\D', '', retweets_text) else 0
                                likes = int(re.sub(r'\D', '', likes_text)) if re.sub(r'\D', '', likes_text) else 0
                            
                            # Parse the date
                            try:
                                date_obj = datetime.strptime(date_str, "%b %d, %Y · %I:%M %p %Z")
                            except:
                                date_obj = datetime.now()
                                
                            tweet_data = {
                                "id": tweet_id,
                                "date": date_obj.isoformat(),
                                "username": username,
                                "content": content,
                                "retweets": retweets,
                                "likes": likes,
                                "source": "Twitter",
                                "query": query
                            }
                            tweets.append(tweet_data)
                            
                        except Exception as e:
                            print(f"Error parsing tweet: {e}")
                            continue
                            
                elif "stocktwits" in source:
                    # Parse Stocktwits format
                    tweet_elements = soup.select(".message")
                    for i, tweet_el in enumerate(tweet_elements):
                        if i >= limit:
                            break
                            
                        try:
                            content_el = tweet_el.select_one(".message__body")
                            username_el = tweet_el.select_one(".user-username")
                            date_el = tweet_el.select_one(".message__time")
                            
                            if not all([content_el, username_el]):
                                continue
                                
                            content = content_el.get_text().strip()
                            username = username_el.get_text().strip()
                            tweet_id = f"stocktwit_{i}"
                            
                            # Get approximate date
                            date_obj = datetime.now()
                            if date_el:
                                date_text = date_el.get_text().strip()
                                # Simple parsing of relative dates like "2h ago"
                                if "h ago" in date_text:
                                    hours = int(date_text.split("h")[0])
                                    date_obj = date_obj.replace(hour=date_obj.hour - hours)
                            
                            tweet_data = {
                                "id": tweet_id,
                                "date": date_obj.isoformat(),
                                "username": username,
                                "content": content,
                                "retweets": 0,  # Not available on Stocktwits
                                "likes": 0,     # Not available on Stocktwits
                                "source": "Stocktwits",
                                "query": query
                            }
                            tweets.append(tweet_data)
                            
                        except Exception as e:
                            print(f"Error parsing Stocktwits message: {e}")
                            continue
                
                elif "tradingview" in source:
                    # Parse TradingView format
                    idea_elements = soup.select(".tv-widget-idea")
                    for i, idea_el in enumerate(idea_elements):
                        if i >= limit:
                            break
                            
                        try:
                            content_el = idea_el.select_one(".tv-widget-idea__description-row")
                            username_el = idea_el.select_one(".tv-widget-idea__author-username")
                            
                            if not all([content_el, username_el]):
                                continue
                                
                            content = content_el.get_text().strip()
                            username = username_el.get_text().strip()
                            tweet_id = f"tradingview_{i}"
                            
                            tweet_data = {
                                "id": tweet_id,
                                "date": datetime.now().isoformat(),
                                "username": username,
                                "content": content,
                                "retweets": 0,  # Not available on TradingView
                                "likes": 0,     # Not available on TradingView
                                "source": "TradingView",
                                "query": query
                            }
                            tweets.append(tweet_data)
                            
                        except Exception as e:
                            print(f"Error parsing TradingView idea: {e}")
                            continue
                
                if tweets:
                    print(f"Successfully scraped {len(tweets)} tweets from {source}")
                    if len(tweets) >= limit:
                        return tweets[:limit]
                    
            else:
                print(f"Failed to access {source}, status code: {response.status_code}")
                
        except Exception as e:
            print(f"Error scraping from {source}: {e}")
            continue
    
    # If we couldn't get enough tweets from any source
    if not tweets:
        print(f"Warning: Could not scrape any tweets for {query}. Falling back to sample data.")
        tweets = generate_sample_tweets(query, limit)
    elif len(tweets) < limit:
        print(f"Warning: Only scraped {len(tweets)} tweets for {query}, needed {limit}. Adding sample data.")
        tweets.extend(generate_sample_tweets(query, limit - len(tweets)))
        
    return tweets[:limit]


def generate_sample_tweets(query: str, limit: int = 100) -> list[dict]:
    """
    Generate sample tweets as a fallback when scraping fails.
    
    Args:
        query: The search query to generate sample tweets for (e.g., "$SOL lang:en")
        limit: Number of sample tweets to generate
        
    Returns:
        A list of dictionaries containing tweet data
    """
    tweets = []
    print(f"Generating {limit} sample tweets for {query} (fallback)...")
    
    # Define some sample usernames
    usernames = ["crypto_fan", "investor123", "trader_pro", "blockchain_dev", 
                "market_watcher", "hodl_king", "finance_guru", "tech_analyst"]
    
    # Define sample content templates based on query
    if "$SOL" in query:
        content_templates = [
            "$SOL is looking bullish today! Price target: ${price}",
            "Just bought more $SOL at ${price}. This is going to the moon! 🚀",
            "$SOL has strong support at ${price}. Holding for the long term.",
            "Technical analysis shows $SOL could reach ${target} by next month.",
            "Not financial advice, but $SOL looks undervalued at current price.",
            "$SOL ecosystem is growing fast. Bullish on this project.",
            "Bearish on $SOL short term, but long term potential is huge.",
            "Solana's TPS is incredible. $SOL deserves to be in the top 3.",
            "$SOL facing resistance at ${price}. Might consolidate before next move.",
            "Comparing $SOL to other L1s, it's clearly superior in speed and cost."
        ]
        # SOL price range ($40-$150)
        price_min, price_max = 40, 150
    else:  # Nifty 50
        content_templates = [
            "Nifty 50 closed at {price} points today. Expecting continued growth.",
            "The Nifty 50 is showing strong momentum. Target: {price} by month end.",
            "Bearish on Nifty 50 short term due to global factors. Support at {price}.",
            "Nifty 50 technical indicators suggest a bullish trend continuing.",
            "IT stocks pushing Nifty 50 higher today. Index up by {change}%.",
            "Banking stocks dragging Nifty 50 down. Index down {change}%.",
            "Nifty 50 near all-time high. Can it break {price} resistance?",
            "Foreign investors buying into Nifty 50 stocks. Bullish signal.",
            "Nifty 50 PE ratio suggests market might be overvalued at current levels.",
            "Nifty 50 showing strong support at {price}. Good buying opportunity."
        ]
        # Nifty 50 price range (18000-22000)
        price_min, price_max = 18000, 22000
    
    # Generate random tweets
    now = datetime.now()
    for i in range(limit):
        # Create content from template
        template = random.choice(content_templates)
        price = round(random.uniform(price_min, price_max), 2)
        target = round(price * random.uniform(1.1, 1.5), 2)
        change = round(random.uniform(0.1, 2.5), 2)
        
        content = template.replace("{price}", str(price)).replace("{target}", str(target)).replace("{change}", str(change))
        
        tweet_data = {
            "id": f"sample_{i}_{int(time.time())}",
            "date": now.isoformat(),
            "username": random.choice(usernames),
            "content": content,
            "retweets": random.randint(0, 500),
            "likes": random.randint(0, 1000),
            "source": "Sample",
            "query": query
        }
        tweets.append(tweet_data)
    
    return tweets


def save_to_json(tweets: list[dict], filename: str):
    """
    Save the list of tweets to a JSON file.
    
    Args:
        tweets: List of tweet dictionaries to save
        filename: Name of the file to save the tweets to
    """
    with open(filename, 'w', encoding='utf-8') as f:
        json.dump(tweets, f, ensure_ascii=False, indent=4)
    
    file_size = os.path.getsize(filename) / 1024  # Size in KB
    print(f"Saved {len(tweets)} tweets to {filename} ({file_size:.2f} KB)")


def main():
    """
    Main function to scrape and save tweets for $SOL and Nifty 50.
    """
    print("Starting tweet data collection...")
    today = datetime.now().strftime('%Y-%m-%d')
    
    # Scrape $SOL tweets
    sol_query = "$SOL lang:en"
    sol_tweets = get_tweets(sol_query)
    sol_filename = f"SOL_{today}.json"
    save_to_json(sol_tweets, sol_filename)
    
    # Scrape Nifty 50 tweets
    nifty_query = "nifty 50 lang:en"
    nifty_tweets = get_tweets(nifty_query)
    nifty_filename = f"NIFTY_{today}.json"
    save_to_json(nifty_tweets, nifty_filename)
    
    # Report scraping results
    real_sol_tweets = sum(1 for t in sol_tweets if t["source"] != "Sample")
    real_nifty_tweets = sum(1 for t in nifty_tweets if t["source"] != "Sample")
    
    print(f"\nScraping results:")
    print(f"$SOL: {real_sol_tweets} real tweets, {len(sol_tweets) - real_sol_tweets} sample tweets")
    print(f"Nifty 50: {real_nifty_tweets} real tweets, {len(nifty_tweets) - real_nifty_tweets} sample tweets")
    print(f"Files saved: {sol_filename}, {nifty_filename}")


if __name__ == "__main__":
    main() 