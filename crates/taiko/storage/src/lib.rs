//! Collection of traits and trait implementations for taiko database operations.

#![doc(
    html_logo_url = "https://raw.githubusercontent.com/paradigmxyz/reth/main/assets/reth-docs.png",
    html_favicon_url = "https://avatars0.githubusercontent.com/u/97369466?s=256",
    issue_tracker_base_url = "https://github.com/paradigmxyz/reth/issues/"
)]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]

pub mod l1_origin;
pub use l1_origin::*;

/// The trait for providing taiko database operations.
pub trait TaikoProvider: L1OriginReader + L1OriginWriter {}

impl<T> TaikoProvider for T where T: L1OriginReader + L1OriginWriter {}
