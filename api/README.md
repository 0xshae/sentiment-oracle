# Sentiment Oracle API & Frontend

This directory contains two modules of the Sentiment Oracle project:

1. **Module 4: REST API** - A Rust API built with Actix Web to serve sentiment data and verify signatures
2. **Module 5: Frontend** - A JavaScript/HTML dashboard to visualize the sentiment data

## Module 4: REST API

The REST API is built with Rust and Actix Web, providing endpoints to access sentiment data and verify signatures.

### Endpoints

- **GET /latest?asset=$SOL** - Returns the latest sentiment data for the specified asset
- **GET /history?asset=$SOL** - Returns historical sentiment data for the specified asset
- **POST /verify** - Verifies a signature against payload data
- **GET /dashboard** - Serves a simple HTML dashboard

### Running the API

```bash
# Set up the environment
cd api
cp .env.example .env  # Adjust settings if needed

# Build and run the API
cargo run
```

The API will be available at http://localhost:8080 by default.

### Testing the API

Use the provided PowerShell script to test the API:

```bash
./test_api.ps1
```

## Module 5: Frontend Dashboard

The frontend dashboard provides a visual interface for the sentiment data.

### Features

- Current sentiment display
- Sentiment timeline visualization
- Signature verification tool

### Running the Frontend

The frontend can be accessed in two ways:

1. Use the API's built-in dashboard at http://localhost:8080/dashboard
2. Serve the frontend files directly:

```bash
# From the api directory
cd frontend
# Serve with any static file server
python -m http.server 8000
```

Then visit http://localhost:8000 in your browser.

## Integration with Other Modules

This API and frontend integrate with the other modules of the Sentiment Oracle project:

- **Module 1: Twitter Scraper** - Provides tweet data
- **Module 2: Sentiment Analysis** - Analyzes tweet sentiment
- **Module 3: Oracle Publisher** - Signs sentiment data and stores it

The API reads the signed sentiment data produced by Module 3 and serves it through REST endpoints for the frontend to visualize.

## Security Features

- CORS enabled for frontend access
- Signature verification using ED25519
- Hash verification using SHA-256 