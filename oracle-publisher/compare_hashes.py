#!/usr/bin/env python3
"""
Script to compare hashes of original and tampered data
"""

import json
import hashlib

def hash_data(data):
    """Hash data and return hex digest"""
    canonical_json = json.dumps(data, sort_keys=True)
    hash_object = hashlib.sha256(canonical_json.encode('utf-8'))
    return hash_object.hexdigest()

def load_json(filename):
    """Load JSON data from file"""
    with open(filename, 'r') as f:
        return json.load(f)

def main():
    print("=== Comparing Original vs. Tampered Data ===\n")
    
    # Load original data
    try:
        original_data = load_json("signed_sentiment_original.json")
        print("Original data loaded successfully.")
        original_hash = hash_data(original_data["data"])
        print(f"Original data hash: {original_hash}")
        print(f"Original label: {original_data['data']['label']}")
    except Exception as e:
        print(f"Error loading original data: {e}")
        return

    # Load tampered data
    try:
        tampered_data = load_json("signed_sentiment.json")
        print("\nTampered data loaded successfully.")
        tampered_hash = hash_data(tampered_data["data"])
        print(f"Tampered data hash: {tampered_hash}")
        print(f"Tampered label: {tampered_data['data']['label']}")
    except Exception as e:
        print(f"Error loading tampered data: {e}")
        return

    # Compare hashes
    print("\nHash comparison:")
    if original_hash == tampered_hash:
        print("✅ Hashes match - data is identical")
    else:
        print("❌ Hashes differ - data has been modified")
        
    # Show what changed
    print("\nChanges detected:")
    differences_found = False
    for key in original_data["data"]:
        if original_data["data"][key] != tampered_data["data"][key]:
            differences_found = True
            print(f"  - Field '{key}' changed from '{original_data['data'][key]}' to '{tampered_data['data'][key]}'")
    
    if not differences_found:
        print("  No differences found in the data fields.")
        
    print("\n=== Conclusion ===")
    print("The signature verification fails because the data hash has changed.")
    print("This demonstrates the security feature of the oracle system:")
    print("Any tampering with the signed data will be detected during verification.")

if __name__ == "__main__":
    main() 