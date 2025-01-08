//! Error types for the Taiko payload builder.

///Taiko specific payload building errors.
#[derive(Debug, thiserror::Error)]
pub enum TaikoPayloadBuilderError {
    /// Thrown when a transaction is not able to be decoded from the payload.
    #[error("failed to decode tx")]
    FailedToDecodeTx,
    /// Thrown when a transaction is not able to be executed as an anchor.
    #[error("missing l1 origin")]
    MissingL1Origin,
}
