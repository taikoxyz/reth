//! Taiko arguments

use clap::Args;
/// Parameters for debugging purposes
#[derive(Debug, Default, Clone, Args, PartialEq, Eq)]
#[command(next_help_heading = "Taiko")]
pub struct TaikoArgs {
    /// The URL of the preconf forwarding server
    #[arg(long = "taiko.preconf-forwarding-server", default_value = None)]
    pub preconf_forwarding_server: Option<String>,
}
