#!/usr/bin/env python3
"""
Script to verify if signed data has been tampered with
"""

import json
import base64
import hashlib
from cryptography.hazmat.primitives.asymmetric import ed25519

def hash_sentiment_data(sentiment_data):
    """Hash sentiment data to create a fixed-length message for signing"""
    # Convert to canonical JSON string
    canonical_json = json.dumps(sentiment_data, sort_keys=True)
    
    # Hash using SHA-256
    hash_object = hashlib.sha256(canonical_json.encode('utf-8'))
    hash_hex = hash_object.hexdigest()
    print(f"Generated hash: {hash_hex}")
    return hash_object.digest()

def verify_signed_data(filename):
    """Verify the signature on signed sentiment data"""
    with open(filename, 'r') as f:
        signed_data = json.load(f)
    
    # Extract components
    sentiment_data = signed_data["data"]
    print(f"Data to verify: {json.dumps(sentiment_data, indent=2)}")
    
    signature = base64.b64decode(signed_data["signature"])
    print(f"Signature (hex): {signature.hex()}")
    
    public_key = base64.b64decode(signed_data["public_key"])
    print(f"Public key (hex): {public_key.hex()}")
    
    # Hash the sentiment data
    data_hash = hash_sentiment_data(sentiment_data)
    
    # Verify the signature
    public_key_obj = ed25519.Ed25519PublicKey.from_public_bytes(public_key)
    try:
        public_key_obj.verify(signature, data_hash)
        return True
    except Exception as e:
        print(f"Verification error: {e}")
        return False

if __name__ == "__main__":
    signed_file = "signed_sentiment.json"
    print(f"Verifying signature in {signed_file}...")
    
    is_valid = verify_signed_data(signed_file)
    
    if is_valid:
        print("✅ Signature is valid! The data was signed by the correct oracle and hasn't been tampered with.")
    else:
        print("❌ Signature is invalid! The data may have been tampered with or signed by a different oracle.") 