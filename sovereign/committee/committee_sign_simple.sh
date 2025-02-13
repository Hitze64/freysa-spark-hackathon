#!/bin/bash
set -euo pipefail
source .env

# Initialize revoke flag
REVOKE=false

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --revoke)
            REVOKE=true
            shift
            ;;
        *)
            INPUT="$*"
            break
            ;;
    esac
done

# If no input provided, read from stdin
if [ -z "${INPUT:-}" ]; then
    INPUT=$(cat)
fi

echo "Debug: Input received:"
echo "$INPUT"

# Extract just the PCR values using grep
PCR0=$(echo "$INPUT" | grep -o 'PCR0:[[:space:]]*[a-f0-9]\{96\}' | grep -o '[a-f0-9]\{96\}')
PCR1=$(echo "$INPUT" | grep -o 'PCR1:[[:space:]]*[a-f0-9]\{96\}' | grep -o '[a-f0-9]\{96\}')
PCR2=$(echo "$INPUT" | grep -o 'PCR2:[[:space:]]*[a-f0-9]\{96\}' | grep -o '[a-f0-9]\{96\}')

echo "Debug: Extracted values:"
echo "PCR0: $PCR0"
echo "PCR1: $PCR1"
echo "PCR2: $PCR2"

# Verify we got all PCR values
if [[ -z "$PCR0" ]] || [[ -z "$PCR1" ]] || [[ -z "$PCR2" ]]; then
    echo "Error: Could not extract all PCR values"
    exit 1
fi

# Build PCR string based on revoke flag
if [ "$REVOKE" = true ]; then
    PCR_STRING="REVOKE: AWS-CODE:$PCR0:$PCR1:$PCR2"
else
    PCR_STRING="AWS-CODE:$PCR0:$PCR1:$PCR2"
fi

echo -e "\nSigning PCR measurements with Safe..."
echo "Using PCR string: $PCR_STRING"

# Run pnpm from the signing directory
pnpm -C signing start "$SAFE_ADDRESS" "$OWNER_PRIVATE_KEY" "$PCR_STRING"
