// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "forge-std/Script.sol";
import "forge-std/console2.sol";

import "../contracts/examples/xERC20.sol";

contract XTransfer is Script {
    // The deployed contract address (will be the same on both chains due to deterministic deployment)
    address constant TOKEN_ADDRESS = 0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512;

    address ALICE = 0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266; //Can stay as is - test values anyways
    address BOB = 0x70997970C51812dc3A010C7d01b50e0d17dc79C8; //Can stay as is - test values anyways
    uint256 ALICE_PK = 0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80;//Can stay as is - test values anyways

    function setUp() public {}

    function run() public {
        address alice = vm.addr(ALICE_PK);

        console.log("\n=== Before Transfer ===");
        //checkBalances(); -> EXPLORER

        vm.startBroadcast(ALICE_PK);

        // Transfer 666 tokens to Bob on L2B (chainId: 167011)
        xERC20(TOKEN_ADDRESS).xTransfer(167010, BOB, 666);

        vm.stopBroadcast();

        console.log("\n=== After Transfer ===");
        //checkBalances(); -> EXPLORER
    }
}