#!/usr/bin/env python3
"""
Sentiment Oracle Demo - A simplified demonstration of the oracle functionality

This script demonstrates the core functionality of the Oracle Publisher:
1. Loading sentiment data from JSON files
2. Hashing the data
3. Signing the hash with a keypair
4. Verifying the signature against the public key
"""

import json
import hashlib
import base64
import os
from cryptography.hazmat.primitives.asymmetric import ed25519
from cryptography.hazmat.primitives import serialization


def generate_keypair():
    """Generate a new ED25519 keypair"""
    private_key = ed25519.Ed25519PrivateKey.generate()
    public_key = private_key.public_key()
    
    # Serialize keys to bytes
    private_bytes = private_key.private_bytes(
        encoding=serialization.Encoding.Raw,
        format=serialization.PrivateFormat.Raw,
        encryption_algorithm=serialization.NoEncryption()
    )
    
    public_bytes = public_key.public_bytes(
        encoding=serialization.Encoding.Raw,
        format=serialization.PublicFormat.Raw
    )
    
    return {
        "private_key": private_bytes,
        "public_key": public_bytes
    }


def save_keypair(keypair, filename):
    """Save a keypair to a file"""
    # Encode bytes as base64 for easier storage
    encoded_keypair = {
        "private_key": base64.b64encode(keypair["private_key"]).decode('utf-8'),
        "public_key": base64.b64encode(keypair["public_key"]).decode('utf-8')
    }
    
    with open(filename, 'w') as f:
        json.dump(encoded_keypair, f, indent=2)
    
    print(f"Keypair saved to {filename}")
    print(f"Public key: {encoded_keypair['public_key']}")


def load_keypair(filename):
    """Load a keypair from a file"""
    with open(filename, 'r') as f:
        encoded_keypair = json.load(f)
    
    # Decode from base64
    keypair = {
        "private_key": base64.b64decode(encoded_keypair["private_key"]),
        "public_key": base64.b64decode(encoded_keypair["public_key"])
    }
    
    return keypair


def hash_sentiment_data(sentiment_data):
    """Hash sentiment data to create a fixed-length message for signing"""
    # Convert to canonical JSON string
    canonical_json = json.dumps(sentiment_data, sort_keys=True)
    
    # Hash using SHA-256
    hash_object = hashlib.sha256(canonical_json.encode('utf-8'))
    return hash_object.digest()


def sign_data(data_hash, private_key_bytes):
    """Sign a hash using the private key"""
    private_key = ed25519.Ed25519PrivateKey.from_private_bytes(private_key_bytes)
    signature = private_key.sign(data_hash)
    return signature


def verify_signature(data_hash, signature, public_key_bytes):
    """Verify a signature using the public key"""
    public_key = ed25519.Ed25519PublicKey.from_public_bytes(public_key_bytes)
    try:
        public_key.verify(signature, data_hash)
        return True
    except Exception:
        return False


def load_sentiment_data(filename):
    """Load sentiment data from a JSON file"""
    with open(filename, 'r') as f:
        sentiment_data = json.load(f)
    return sentiment_data


def save_signed_data(sentiment_data, signature, public_key, filename):
    """Save the signed sentiment data to a file"""
    signed_data = {
        "data": sentiment_data,
        "signature": base64.b64encode(signature).decode('utf-8'),
        "public_key": base64.b64encode(public_key).decode('utf-8')
    }
    
    with open(filename, 'w') as f:
        json.dump(signed_data, f, indent=2)
    
    print(f"Signed data saved to {filename}")


def verify_signed_data(filename):
    """Verify the signature on signed sentiment data"""
    with open(filename, 'r') as f:
        signed_data = json.load(f)
    
    # Extract components
    sentiment_data = signed_data["data"]
    signature = base64.b64decode(signed_data["signature"])
    public_key = base64.b64decode(signed_data["public_key"])
    
    # Hash the sentiment data
    data_hash = hash_sentiment_data(sentiment_data)
    
    # Verify the signature
    is_valid = verify_signature(data_hash, signature, public_key)
    
    return is_valid


def main():
    """Main function to demonstrate the oracle functionality"""
    print("Sentiment Oracle Demo")
    print("====================")
    
    # Check if keypair file exists, if not, generate one
    keypair_file = "oracle_keypair.json"
    if not os.path.exists(keypair_file):
        print("Generating new keypair...")
        keypair = generate_keypair()
        save_keypair(keypair, keypair_file)
    else:
        print(f"Loading existing keypair from {keypair_file}...")
        keypair = load_keypair(keypair_file)
    
    # Load sentiment data
    sentiment_file = "sample_sentiment.json"
    print(f"Loading sentiment data from {sentiment_file}...")
    sentiment_data = load_sentiment_data(sentiment_file)
    
    # Hash the sentiment data
    print("Hashing sentiment data...")
    data_hash = hash_sentiment_data(sentiment_data)
    print(f"Hash: {data_hash.hex()}")
    
    # Sign the hash
    print("Signing hash with private key...")
    signature = sign_data(data_hash, keypair["private_key"])
    print(f"Signature: {signature.hex()}")
    
    # Save the signed data
    signed_file = "signed_sentiment.json"
    print(f"Saving signed data to {signed_file}...")
    save_signed_data(sentiment_data, signature, keypair["public_key"], signed_file)
    
    # Verify the signature
    print("Verifying signature...")
    is_valid = verify_signed_data(signed_file)
    
    if is_valid:
        print("✅ Signature is valid! The data was signed by the correct oracle and hasn't been tampered with.")
    else:
        print("❌ Signature is invalid! The data may have been tampered with or signed by a different oracle.")
    
    print("\nWhat this demonstrates:")
    print("1. The oracle generates a keypair (or uses an existing one)")
    print("2. The oracle hashes the sentiment data to create a fixed-length message")
    print("3. The oracle signs the hash with its private key")
    print("4. The signed data is stored with the signature and public key")
    print("5. Anyone can verify that the data was signed by the oracle and hasn't been modified")
    
    print("\nIn a production system:")
    print("- The keypair would be generated once and kept securely")
    print("- The signed data would be stored on-chain (e.g., Solana)")
    print("- Multiple oracles might sign the same data, allowing users to trust data with multiple signatures")


if __name__ == "__main__":
    main() 