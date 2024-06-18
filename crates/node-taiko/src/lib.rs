//! Standalone crate for ethereum-specific Reth configuration and builder types.

#![doc(
    html_logo_url = "https://raw.githubusercontent.com/paradigmxyz/reth/main/assets/reth-docs.png",
    html_favicon_url = "https://avatars0.githubusercontent.com/u/97369466?s=256",
    issue_tracker_base_url = "https://github.com/paradigmxyz/reth/issues/"
)]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]

/// Exports commonly used concrete instances of the [EngineTypes](reth_node_api::EngineTypes)
/// trait.
pub mod engine;
pub use engine::TaikoEngineTypes;

/// Exports commonly used concrete instances of the
/// [ConfigureEvmEnv](reth_node_api::ConfigureEvmEnv) trait.
pub mod evm;
pub use evm::TaikoEvmConfig;
pub mod node;
pub use node::TaikoNode;
