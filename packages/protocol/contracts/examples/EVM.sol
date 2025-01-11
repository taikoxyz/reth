// SPDX-License-Identifier: MIT

pragma solidity >=0.8.12 <0.9.0;
// EVM library
library EVM {
    address constant xCallOptionsAddress = address(0x04D2);

    uint constant l1ChainId = 1;
    uint16 constant version = 1;

    function xCallOnL1()
        internal
        view
    {
        xCallOptions(l1ChainId);
    }

    function xCallOnL1(bool sandbox)
        internal
        view
    {
        xCallOptions(l1ChainId, sandbox);
    }

    function xCallOptions(uint chainID)
        internal
        view
    {
        xCallOptions(chainID, false);
    }

    function xCallOptions(uint chainID, bool sandbox)
        internal
        view
    {
        xCallOptions(chainID, sandbox, tx.origin, address(this));
    }

    function xCallOptions(uint chainID, bool sandbox, address txOrigin, address msgSender)
        internal
        view
    {
        xCallOptions(chainID, sandbox, txOrigin, msgSender, 0x0, "");
    }

    function xCallOptions(uint chainID, bool sandbox, bytes32 blockHash, bytes memory proof)
        internal
        view
    {
        xCallOptions(chainID, sandbox, address(0), address(0), blockHash, proof);
    }

    function xCallOptions(uint chainID, bool sandbox, address txOrigin, address msgSender, bytes32 blockHash, bytes memory proof)
        internal
        view
    {
        // require(chainID() != l1ChainId);

        bytes memory input = abi.encodePacked(version, uint64(chainID), sandbox, txOrigin, msgSender, blockHash, proof);
        (bool success, ) = xCallOptionsAddress.staticcall(input); //delegatecall
        require(success);
    }

    function isOnL1() internal view returns (bool) {
        return chainId() == l1ChainId;
    }

    function chainId() internal view returns (uint256) {
        return block.chainid;
    }
}