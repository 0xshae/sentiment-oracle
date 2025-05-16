#!/usr/bin/env pwsh
# Test script for the Sentiment Oracle API

$apiBase = "http://localhost:8080"

Write-Host "Testing Sentiment Oracle API at $apiBase" -ForegroundColor Cyan

# Function to safely make API calls
function Invoke-SafeRequest {
    param (
        [string]$Uri,
        [string]$Method = "Get",
        [string]$Body = $null,
        [string]$ContentType = "application/json"
    )

    try {
        if ($Method -eq "Get") {
            $response = Invoke-RestMethod -Uri $Uri -Method $Method -TimeoutSec 5
        } else {
            $response = Invoke-RestMethod -Uri $Uri -Method $Method -Body $Body -ContentType $ContentType -TimeoutSec 5
        }
        return $response
    } catch {
        Write-Host "Error connecting to API: $_" -ForegroundColor Red
        return $null
    }
}

# Test 1: Get latest sentiment
Write-Host "`n1. Testing GET /latest?asset=`$SOL" -ForegroundColor Yellow
$response = Invoke-SafeRequest -Uri "$apiBase/latest?asset=`$SOL" 
if ($response) {
    $response | ConvertTo-Json -Depth 10
} else {
    Write-Host "No response received from the server" -ForegroundColor Red
}

# Wait a moment between requests
Start-Sleep -Seconds 1

# Test 2: Get sentiment history
Write-Host "`n2. Testing GET /history?asset=`$SOL" -ForegroundColor Yellow
$response = Invoke-SafeRequest -Uri "$apiBase/history?asset=`$SOL"
if ($response) {
    $response | ConvertTo-Json -Depth 10
} else {
    Write-Host "No response received from the server" -ForegroundColor Red
}

# Wait a moment between requests
Start-Sleep -Seconds 1

# Test 3: Verify signature
Write-Host "`n3. Testing POST /verify with valid data" -ForegroundColor Yellow

# Load the original data from signed_sentiment.json
$signedDataPath = "../oracle-publisher/signed_sentiment.json"
if (Test-Path $signedDataPath) {
    $signedData = Get-Content -Path $signedDataPath | ConvertFrom-Json

    # Construct the verify request
    $verifyRequest = @{
        payload = $signedData.data
        signature = $signedData.signature
        signer = $signedData.public_key
    }

    # Convert to JSON and send the request
    $verifyJson = $verifyRequest | ConvertTo-Json -Depth 10
    $response = Invoke-SafeRequest -Uri "$apiBase/verify" -Method "Post" -Body $verifyJson
    if ($response) {
        $response | ConvertTo-Json -Depth 10
    } else {
        Write-Host "No response received from the server" -ForegroundColor Red
    }

    # Wait a moment between requests
    Start-Sleep -Seconds 1

    # Test 4: Verify signature with tampered data
    Write-Host "`n4. Testing POST /verify with tampered data" -ForegroundColor Yellow

    # Tamper with the data by changing the label
    $tamperedData = $signedData.PSObject.Copy()
    $tamperedData.data.label = "NEGATIVE"

    # Construct the verify request with tampered data
    $verifyRequest = @{
        payload = $tamperedData.data
        signature = $signedData.signature
        signer = $signedData.public_key
    }

    # Convert to JSON and send the request
    $verifyJson = $verifyRequest | ConvertTo-Json -Depth 10
    $response = Invoke-SafeRequest -Uri "$apiBase/verify" -Method "Post" -Body $verifyJson
    if ($response) {
        $response | ConvertTo-Json -Depth 10
    } else {
        Write-Host "No response received from the server" -ForegroundColor Red
    }
} else {
    Write-Host "Error: Could not find signed_sentiment.json at $signedDataPath" -ForegroundColor Red
}

Write-Host "`nAPI Test Complete" -ForegroundColor Green