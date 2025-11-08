#!/bin/bash

# Blend Vault Deployment Script for Stellar Mainnet
# This script builds, deploys, and initializes the Blend Vault contract

set -e  # Exit on any error

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Configuration - Update these values before deployment
NETWORK="mainnet"
SOURCE_ACCOUNT="${STELLAR_ACCOUNT:-james}"  # Set via environment variable

USDC_ADDRESS="${USDC_ADDRESS:-CCW67TSZV3SSS2HXMBQ5JFGCKJNXKZM7UQUWUZPUTHXSTZLEO7SJMI75}"
BLEND_POOL="${BLEND_POOL:-CCCCIQSDILITHMM7PBSLVDT5MISSY7R26MNZXCX4H7J5JQ5FPIYOGYFS}"
USDC_RESERVE_INDEX="${USDC_RESERVE_INDEX:-1}"  # USDC is at index 1 in Blend pool reserve list
BLND_TOKEN="${BLND_TOKEN:-CD25MNVTZDL4Y3XBCPCJXGXATV5WUHHOWMYFF4YBEGU5FCPGMYTVG5JY}"
BLND_RESERVE_INDEX="${BLND_RESERVE_INDEX:-3}"  # reserve_token_id for USDC supply = 1*2+1 = 3
COMET_POOL="${COMET_POOL:-CAS3FL6TLZKDGGSISDBWGGPXT3NRR4DYTZD7YOD3HMYO6LTJUVGRVEAM}"

DECIMALS_OFFSET=0  # Same decimals as USDC (7)

# WASM output path
WASM_PATH="target/wasm32v1-none/release/blend_vault.wasm"

# Step 1: Clean previous builds
 echo -e "${YELLOW}Step 1: Cleaning previous builds...${NC}"
 cargo clean

# Step 2: Build the contract
echo -e "${YELLOW}Step 2: Building contract for WASM target...${NC}"
stellar contract build

# Verify WASM file exists
if [ ! -f "$WASM_PATH" ]; then
    echo -e "${RED}Error: WASM file not found at $WASM_PATH${NC}"
    exit 1
fi

echo -e "${GREEN}✓ Build successful${NC}"
echo "WASM size: $(du -h $WASM_PATH | cut -f1)"

# Step 3: Check prerequisites
echo -e "${YELLOW}Step 3: Checking prerequisites...${NC}"

if [ -z "$SOURCE_ACCOUNT" ]; then
    echo -e "${RED}Error: STELLAR_ACCOUNT environment variable not set${NC}"
    echo "Please set your deployer account:"
    echo "  export STELLAR_ACCOUNT=your-account-name"
    exit 1
fi

# Verify stellar CLI is installed
if ! command -v stellar &> /dev/null; then
    echo -e "${RED}Error: stellar CLI not found${NC}"
    echo "Install it from: https://developers.stellar.org/docs/tools/developer-tools/cli/install"
    exit 1
fi

# Verify network configuration
if ! stellar network ls | grep -q "$NETWORK"; then
    echo -e "${RED}Error: Network '$NETWORK' not configured${NC}"
    echo "Configure it with: stellar network add"
    exit 1
fi

# Step 4: Validate configuration
echo -e "${YELLOW}Step 4: Validating configuration...${NC}"

if [ -z "$COMET_POOL" ]; then
    echo -e "${RED}Error: COMET_POOL address not set${NC}"
    echo "Please set the Comet pool address for BLND-USDC swaps"
    exit 1
fi

echo "Configuration:"
echo "  Network: $NETWORK"
echo "  Source Account: $SOURCE_ACCOUNT"
echo "  USDC Address: $USDC_ADDRESS"
echo "  Blend Pool: $BLEND_POOL"
echo "  USDC Reserve Index: $USDC_RESERVE_INDEX"
echo "  BLND Token: $BLND_TOKEN"
echo "  BLND Reserve Index: $BLND_RESERVE_INDEX"
echo "  Comet Pool: $COMET_POOL"
echo "  Decimals Offset: $DECIMALS_OFFSET"
echo ""

#read -p "Proceed with deployment? (y/N) " -n 1 -r
#echo
#if [[ ! $REPLY =~ ^[Yy]$ ]]; then
#    echo "Deployment cancelled"
#    exit 0
#fi

# Step 5: Deploy the contract (without initialization)
echo -e "${YELLOW}Step 5: Deploying contract to $NETWORK (without initialization)...${NC}"
echo "The contract will be deployed without any initialization parameters."
echo "This allows you to inspect the deployment before initializing."

CONTRACT_ID=$(stellar contract deploy \
    --wasm "$WASM_PATH" \
    --source "$SOURCE_ACCOUNT" \
    --network "$NETWORK" \
    2>&1 | tee /dev/tty | tail -n1)

# Check if deployment failed
if [[ "$CONTRACT_ID" == *"error"* ]] || [ -z "$CONTRACT_ID" ]; then
    echo -e "${RED}Error: Deployment failed${NC}"
    echo "Contract ID output: $CONTRACT_ID"
    exit 1
fi

echo -e "${GREEN}✓ Contract deployed (uninitialized)${NC}"
echo "Contract ID: $CONTRACT_ID"
echo ""
echo "NOTE: The contract is now deployed but NOT initialized."
echo "You can verify the deployment before proceeding with initialization."
echo ""

#read -p "Proceed with initialization? (y/N) " -n 1 -r
#echo
#if [[ ! $REPLY =~ ^[Yy]$ ]]; then
#    echo "Initialization skipped. You can initialize later with:"
#    echo "  stellar contract invoke --id $CONTRACT_ID --network $NETWORK -- initialize \\"
#    echo "    --asset $USDC_ADDRESS \\"
#    echo "    --decimals_offset $DECIMALS_OFFSET \\"
#    echo "    --blend_pool $BLEND_POOL \\"
#    echo "    --usdc_reserve_index $USDC_RESERVE_INDEX \\"
#    echo "    --blnd_token $BLND_TOKEN \\"
#    echo "    --blnd_reserve_index $BLND_RESERVE_INDEX \\"
#    echo "    --comet_pool $COMET_POOL"
#    exit 0
#fi

# Step 6: Initialize the contract
echo -e "${YELLOW}Step 6: Initializing contract...${NC}"
echo "NOTE: Initialization can only be done once. Subsequent calls will fail."

stellar contract invoke \
    --id "$CONTRACT_ID" \
    --source "$SOURCE_ACCOUNT" \
    --network "$NETWORK" \
    -- initialize \
    --asset "$USDC_ADDRESS" \
    --decimals_offset "$DECIMALS_OFFSET" \
    --blend_pool "$BLEND_POOL" \
    --usdc_reserve_index "$USDC_RESERVE_INDEX" \
    --blnd_token "$BLND_TOKEN" \
    --blnd_reserve_index "$BLND_RESERVE_INDEX" \
    --comet_pool "$COMET_POOL"

echo -e "${GREEN}✓ Contract initialized${NC}"

# Step 7: Save deployment info
echo -e "${YELLOW}Step 7: Saving deployment information...${NC}"

TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
DEPLOYMENT_FILE="deployment_${TIMESTAMP}.txt"

cat > "$DEPLOYMENT_FILE" << EOF
Blend Vault Deployment
======================
Timestamp: $TIMESTAMP
Network: $NETWORK
Deployer: $SOURCE_ACCOUNT

Contract ID: $CONTRACT_ID

Configuration:
  USDC Address: $USDC_ADDRESS
  Blend Pool: $BLEND_POOL
  USDC Reserve Index: $USDC_RESERVE_INDEX
  BLND Token: $BLND_TOKEN
  BLND Reserve Index: $BLND_RESERVE_INDEX
  Comet Pool: $COMET_POOL
  Decimals Offset: $DECIMALS_OFFSET

Stellar Expert: https://stellar.expert/explorer/public/contract/$CONTRACT_ID
EOF

echo -e "${GREEN}✓ Deployment information saved to $DEPLOYMENT_FILE${NC}"
cat "$DEPLOYMENT_FILE"

echo ""
echo -e "${GREEN}═══════════════════════════════════════${NC}"
echo -e "${GREEN}  Deployment Successful! 🚀${NC}"
echo -e "${GREEN}═══════════════════════════════════════${NC}"
echo ""
echo "Next steps:"
echo "  1. Verify contract on Stellar Expert"
echo "  2. Test deposit/withdraw functionality"
echo "  3. Monitor contract performance"
echo ""
echo "Contract ID: $CONTRACT_ID"
