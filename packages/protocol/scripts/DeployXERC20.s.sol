pragma solidity ^0.8.20;

import "forge-std/Script.sol";
import "forge-std/console2.sol";

import "../contracts/examples/xERC20.sol";

// forge script --rpc-url  http://127.0.0.1:8545 
//script/DeployOnL1.s.sol -vvvv --broadcast --private-key 
// 0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80
contract DeployXERC20 is Script {
    uint256 public adminPrivateKey = 0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80;

    function run() external {
        require(adminPrivateKey != 0, "PRIVATE_KEY not set");

        vm.startBroadcast(adminPrivateKey);

        address xERC20 = address(new xERC20(99999));

        console2.log("xERC20 address is:", xERC20);

        vm.stopBroadcast();
    }
}
