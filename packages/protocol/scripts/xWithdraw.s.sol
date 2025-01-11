// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "forge-std/Script.sol";
import "forge-std/console2.sol";

import "../contracts/examples/xERC20.sol";

contract XTransfer is Script {
    // The deployed contract address (will be the same on both chains due to deterministic deployment)
    address constant TOKEN_ADDRESS = 0x5FbDB2315678afecb367f032d93F642f64180aa3;

    address ALICE = 0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266; //Can stay as is - test values anyways
    address BOB = 0xE25583099BA105D9ec0A67f5Ae86D90e50036425; //Can stay as is - test values anyways
    address CHARLIE = 0x614561D2d143621E126e87831AEF287678B442b8; //Can stay as is - test values anyways
    uint256 ALICE_PK = 0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80;//Can stay as is - test values anyways
    uint256 BOB_PK = 0x39725efee3fb28614de3bacaffe4cc4bd8c436257e2c8bb887c4b5c4be45e76d;//Can stay as is - test values anyways

    function setUp() public {}

    function run() public {
        address bob = vm.addr(BOB_PK);

        vm.startBroadcast();
        // Sending 1 ETH
        (bool success, ) = bob.call{value: 1 ether}("");
        require(success, "Failed to send Ether");
        vm.stopBroadcast();

        console.log("\n=== Before Transfer ===");

        vm.startBroadcast(BOB_PK);

        // Withdraw some tokens to L1
        xERC20(TOKEN_ADDRESS).xTransfer(160010, CHARLIE, 222);

        vm.stopBroadcast();

        console.log("\n=== After Transfer ===");
        //checkBalances(); -> EXPLORER
    }
}