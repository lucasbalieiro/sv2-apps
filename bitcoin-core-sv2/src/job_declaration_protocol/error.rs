//! Error types for the Bitcoin Core IPC integration.

use std::path::Path;
use stratum_core::bitcoin::consensus;

/// Errors from the [`crate::job_declaration_protocol::BitcoinCoreSv2JDP`] layer.
#[derive(Debug)]
pub enum BitcoinCoreSv2JDPError {
    /// Cap'n Proto RPC error.
    CapnpError(capnp::Error),
    /// Failed to connect to the Bitcoin Core Unix socket.
    CannotConnectToUnixSocket(Box<Path>, String),
    /// Failed to deserialize a block from the IPC response.
    FailedToDeserializeBlock(consensus::encode::Error),
    /// Failed to deserialize a block header from the IPC response.
    FailedToDeserializeBlockHeader(consensus::encode::Error),
    /// Readiness signal receiver was dropped before bootstrap completed.
    ReadinessSignalFailed,
}

impl From<capnp::Error> for BitcoinCoreSv2JDPError {
    fn from(error: capnp::Error) -> Self {
        BitcoinCoreSv2JDPError::CapnpError(error)
    }
}
