#!/bin/bash
set -euo pipefail

# Load environment variables
source .env

echo "Cleaning up any old containers..."
docker rm -f nitro-attestation-run 2>/dev/null || true

echo "Building attestation container..."
docker build --no-cache --platform linux/amd64 -t nitro-attestation .

echo -e "\nRunning attestation for:"
echo "Repository: $1"
echo "Commit: $2"
echo -e "\nStarting container...\n"

# Run container and capture PCR string directly
PCR_STRING=$(docker run \
    --name nitro-attestation-run \
    --platform linux/amd64 \
    -v /var/run/docker.sock:/var/run/docker.sock \
    nitro-attestation $@)

echo -e "\nContainer logs:"
docker logs nitro-attestation-run

echo -e "\nCleaning up container..."
docker rm nitro-attestation-run

echo -e "\nSigning PCR measurements with Safe..."
pnpm start "$SAFE_ADDRESS" "$OWNER_PRIVATE_KEY" "$PCR_STRING" | jq .
