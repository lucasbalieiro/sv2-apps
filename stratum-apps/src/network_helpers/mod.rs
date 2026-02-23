//! High-level networking utilities for SV2 connections
//!
//! This module provides connection management, encrypted streams, and protocol handling
//! for Stratum V2 applications. It includes support for:
//!
//! - Noise-encrypted connections ([`noise_connection`], [`noise_stream`])
//! - SV1 protocol connections ([`sv1_connection`]) - when `sv1` feature is enabled
//!
//! Originally from the `network_helpers_sv2` crate.

pub mod noise_connection;
pub mod noise_stream;

#[cfg(feature = "sv1")]
pub mod sv1_connection;

use async_channel::{RecvError, SendError};
use std::{fmt, time::Duration};
use stratum_core::{
    binary_sv2::{Deserialize, GetSize, Serialize},
    codec_sv2::{Error as CodecError, HandshakeRole},
    noise_sv2::{Initiator, Responder},
};
use tokio::net::TcpStream;

use crate::{
    key_utils::{Secp256k1PublicKey, Secp256k1SecretKey},
    network_helpers::noise_stream::NoiseTcpStream,
};

/// Networking errors that can occur in SV2 connections
#[derive(Debug)]
pub enum Error {
    /// Invalid handshake message received from remote peer
    HandshakeRemoteInvalidMessage,
    /// Error from the codec layer
    CodecError(CodecError),
    /// Error receiving from async channel
    RecvError,
    /// Error sending to async channel
    SendError,
    /// Socket was closed, likely by the peer
    SocketClosed,
    /// Handshake timeout
    HandshakeTimeout,
    /// Invalid key provided to construct an Initiator or Responder
    InvalidKey,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::HandshakeRemoteInvalidMessage => {
                write!(f, "Invalid handshake message received from remote peer")
            }

            Error::CodecError(e) => write!(f, "{}", e),

            Error::RecvError => write!(f, "Error receiving from async channel"),

            Error::SendError => write!(f, "Error sending to async channel"),

            Error::SocketClosed => write!(f, "Socket was closed (likely by the peer)"),

            Error::HandshakeTimeout => write!(f, "Handshake timeout"),

            Error::InvalidKey => write!(f, "Invalid key provided for handshake"),
        }
    }
}

impl From<CodecError> for Error {
    fn from(e: CodecError) -> Self {
        Error::CodecError(e)
    }
}

impl From<RecvError> for Error {
    fn from(_: RecvError) -> Self {
        Error::RecvError
    }
}

impl<T> From<SendError<T>> for Error {
    fn from(_: SendError<T>) -> Self {
        Error::SendError
    }
}

/// Default handshake timeout used by [`connect`] and [`accept`].
/// Use [`noise_stream::NoiseTcpStream::new`] directly to override.
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);

/// Connects to an upstream server as a Noise initiator, returning the split read/write halves.
///
/// Pass `Some(key)` to verify the server's authority public key, or `None` to skip
/// verification (encrypted but unauthenticated — use only on trusted networks).
pub async fn connect<Message>(
    stream: TcpStream,
    authority_pub_key: Option<Secp256k1PublicKey>,
) -> Result<NoiseTcpStream<Message>, Error>
where
    Message: Serialize + Deserialize<'static> + GetSize + Send + 'static,
{
    let initiator = match authority_pub_key {
        Some(key) => Initiator::from_raw_k(key.into_bytes()).map_err(|_| Error::InvalidKey)?,
        None => Initiator::without_pk().map_err(|_| Error::InvalidKey)?,
    };
    let stream = noise_stream::NoiseTcpStream::new(
        stream,
        HandshakeRole::Initiator(initiator),
        HANDSHAKE_TIMEOUT,
    )
    .await?;
    Ok(stream)
}

/// Accepts a downstream connection as a Noise responder, returning the split read/write halves.
///
/// `cert_validity` controls how long the generated Noise certificate is valid,
/// which is independent of the handshake timeout.
pub async fn accept<Message>(
    stream: TcpStream,
    pub_key: Secp256k1PublicKey,
    prv_key: Secp256k1SecretKey,
    cert_validity: u64,
) -> Result<NoiseTcpStream<Message>, Error>
where
    Message: Serialize + Deserialize<'static> + GetSize + Send + 'static,
{
    let responder = Responder::from_authority_kp(
        &pub_key.into_bytes(),
        &prv_key.into_bytes(),
        Duration::from_secs(cert_validity),
    )
    .map_err(|_| Error::InvalidKey)?;
    let stream = noise_stream::NoiseTcpStream::new(
        stream,
        HandshakeRole::Responder(responder),
        HANDSHAKE_TIMEOUT,
    )
    .await?;
    Ok(stream)
}
