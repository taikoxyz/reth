// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;


import "../L1/TaikoData.sol";

contract GwynethContract {
    function applyStateDelta(TaikoData.StateDiffStorageSlot[] calldata slots)
        external
    {
        // TODO(Brecht): check msg.sender
        // Run over all state changes
        for (uint256 i = 0; i < slots.length; i++) {
            // Apply the updated state to the storage
            bytes32 key = slots[i].key;
            bytes32 value = slots[i].value;
            // Possible to check the slot against any variable.slot
            // to e.g. throw a custom event
            assembly {
                sstore(key, value)
            }
        }
    }
}