# Uniswap Local Deployment Guide

This guide explains how to deploy and run Uniswap locally or from a local repository using the following components:

- **Smart Contracts**
- **SDK** (with chain support)
- **Interface/UI**

---

## ⚠️ Important Note

The deployment addresses (`FACTORY_ADDRESS`, `WETH`) below are valid **only if the first transactions made with the specified private key (`53321db7c1e331d93a11a41d16f004d7ff63972ec8ec7c25db329728ceeb1710`)** are the Uniswap contract deployments.  
- **Do not use this private key for any other transactions before deploying the Uniswap contracts.**  
- Otherwise, you must update the **Interface** and **SDK** repositories with the new deployment addresses.

---

## 1. Uniswap Smart Contracts

1. Clone the repository:  
   ```bash
   git clone https://github.com/taikoxyz/uniswap-v2
2. Init submodules and build the bindings
   ```bash
   git submodule init
   forge build
3. Deploy the contracts
   ```bash
   $ forge script script/UniswapDeployer.s.sol --rpc-url http://localhost:32002 --broadcast --legacy
   $ forge script script/DeployTokens.s.sol --rpc-url http://localhost:32002 --broadcast --legacy
## 2. Uniswap SDK

1. Clone the repository and switch to the `gwyneth_uniswapV2` branch:
   ```bash
   git clone https://github.com/adaki2004/v2-sdk && cd v2-sdk
   git checkout gwyneth_uniswapV2
2. Build the SDK:
   ```bash
   yarn && yarn build
> **_NOTE:_** Ensure that the contracts are deployed first before interacting with this repository using the specified private key.

## 3. Uniswap Interface/UI
> **_NOTE:_** Ensure that the SDK repository is in the same root directory as one, as it is referenced in `package.json` like this:
`"@uniswap/sdk": "file:../v2-sdk"`.

1. Clone the repository:  
   ```bash
   git clone https://github.com/adaki2004/interface

2. Switch to the `gwyneth_uniswapV2` branch:
   ```bash
   git checkout gwyneth_uniswapV2
3. Install dependencies
   ```bash
   npm install
4. Deploy the contracts
   ```bash
   yarn
   export NODE_OPTIONS=--openssl-legacy-provider
   yarn start
## Additional Notes
Ensure that the repositories are properly structured in your working directory for dependency resolution.
If deployment addresses change, you will need to update the Interface and SDK configurations.