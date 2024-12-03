#!/bin/bash

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Contract address we expect after deployment
CONTRACT_ADDRESS="0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512"

# Function to check if contract is deployed
check_contract_deployment() {
    local rpc_url=$1
    local retries=30  # Number of retries (30 * 2 = 60 seconds max wait)
    local deployed=false

    echo -e "${YELLOW}Waiting for contract deployment confirmation...${NC}"
    
    for i in $(seq 1 $retries); do
        # Using curl to check if the contract code exists at the address
        result=$(curl -s -X POST -H "Content-Type: application/json" --data "{\"jsonrpc\":\"2.0\",\"method\":\"eth_getCode\",\"params\":[\"$CONTRACT_ADDRESS\", \"latest\"],\"id\":1}" $rpc_url)
        
        # Check if the result contains more than just "0x" (empty contract)
        if [[ $result == *"0x60806040"* ]]; then
            deployed=true
            break
        fi
        
        echo -e "${YELLOW}Attempt $i/$retries: Contract not yet deployed, waiting...${NC}"
        sleep 2
    done

    if [ "$deployed" = true ]; then
        echo -e "${GREEN}Contract deployment confirmed!${NC}"
        return 0
    else
        echo -e "${RED}Contract deployment could not be confirmed after $retries attempts${NC}"
        return 1
    fi
}

echo -e "${GREEN}Deploying to L2A...${NC}"
forge script --rpc-url http://127.0.0.1:32005 scripts/DeployXERC20.s.sol -vvvv --broadcast --private-key 0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80 --legacy

if [ $? -eq 0 ]; then
    # Check if contract is deployed on L2A
    if check_contract_deployment "http://127.0.0.1:32005"; then
        echo -e "${GREEN}Verifying L2A contract...${NC}"
        forge verify-contract "$CONTRACT_ADDRESS" "contracts/examples/xERC20.sol:xERC20" --watch --verifier-url "http://localhost:64003/api" --verifier blockscout --chain-id 167010 --libraries contracts/examples/EVM.sol:EVM:0x5FbDB2315678afecb367f032d93F642f64180aa3
    else
        echo -e "${RED}L2A deployment verification failed. Stopping.${NC}"
        exit 1
    fi
else
    echo -e "${RED}L2A deployment failed. Stopping.${NC}"
    exit 1
fi

echo -e "${GREEN}Deploying to L2B...${NC}"
forge script --rpc-url http://127.0.0.1:32006 scripts/DeployXERC20.s.sol -vvvv --broadcast --private-key 0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80 --legacy

if [ $? -eq 0 ]; then
    # Check if contract is deployed on L2B
    if check_contract_deployment "http://127.0.0.1:32006"; then
        echo -e "${GREEN}Verifying L2B contract...${NC}"
        forge verify-contract "$CONTRACT_ADDRESS" "contracts/examples/xERC20.sol:xERC20" --watch --verifier-url "http://localhost:64005/api" --verifier blockscout --chain-id 167011 --libraries contracts/examples/EVM.sol:EVM:0x5FbDB2315678afecb367f032d93F642f64180aa3
    else
        echo -e "${RED}L2B deployment verification failed. Stopping.${NC}"
        exit 1
    fi
else
    echo -e "${RED}L2B deployment failed. Stopping.${NC}"
    exit 1
fi

# Add a delay before xTransfer to ensure everything is ready
echo -e "${YELLOW}Waiting 5 seconds before executing xTransfer...${NC}"
sleep 5

echo -e "${GREEN}Executing xTransfer...${NC}"
forge script scripts/XTransfer.s.sol --rpc-url http://127.0.0.1:32005 -vvvv --broadcast --private-key 0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80 --legacy