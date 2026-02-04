//! ## Configuration Module
//!
//! Defines [`JDSConfig`], the configuration structure for the Job Declaration Server (JDS), along
//! with its supporting types.
//!
//! This module handles:
//! - Initializing [`JDSConfig`]
//! - Managing [`JobValidationEngineConfig`]
//! - Validating and converting coinbase outputs

use std::{net::SocketAddr, path::PathBuf};
use stratum_apps::{
    config_helpers::{opt_path_from_toml, CoinbaseRewardScript},
    key_utils::{Secp256k1PublicKey, Secp256k1SecretKey},
    tp_type::BitcoinNetwork,
};

/// Partial configuration deserialized from the `[jds]` TOML section, when embedded in Pool.
///
/// Contains only JDS-specific fields. Shared fields (authority keys, cert validity,
/// coinbase script and engine config) are inherited from the parent Pool config.
#[derive(Clone, Debug, serde::Deserialize)]
pub struct JDSPartialConfig {
    listen_address: SocketAddr,
    #[serde(default)]
    supported_extensions: Vec<u16>,
    #[serde(default)]
    required_extensions: Vec<u16>,
}

/// Complete JDS configuration with all required fields populated.
///
/// Built from [`JDSPartialConfig`] combined with Pool config values.
/// All fields are non-optional and ready to use.
#[derive(Clone, Debug)]
pub struct JDSConfig {
    listen_address: SocketAddr,
    engine_config: JobValidationEngineConfig,
    authority_public_key: Secp256k1PublicKey,
    authority_secret_key: Secp256k1SecretKey,
    cert_validity_sec: u64,
    coinbase_reward_script: CoinbaseRewardScript,
    supported_extensions: Vec<u16>,
    required_extensions: Vec<u16>,
}

impl JDSPartialConfig {
    /// Constructs a minimal [`JDSPartialConfig`] with just the listen address.
    ///
    /// All other fields use defaults. This is typically used for programmatic construction
    /// (e.g., in tests) rather than TOML deserialization.
    pub fn new(listen_address: SocketAddr) -> Self {
        Self {
            listen_address,
            supported_extensions: Vec::new(),
            required_extensions: Vec::new(),
        }
    }
}

#[cfg_attr(not(test), hotpath::measure_all)]
impl JDSConfig {
    /// Constructs a complete [`JDSConfig`] directly (typically for tests).
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        listen_address: SocketAddr,
        engine_config: JobValidationEngineConfig,
        authority_public_key: Secp256k1PublicKey,
        authority_secret_key: Secp256k1SecretKey,
        cert_validity_sec: u64,
        coinbase_reward_script: CoinbaseRewardScript,
        supported_extensions: Vec<u16>,
        required_extensions: Vec<u16>,
    ) -> Self {
        Self {
            listen_address,
            engine_config,
            authority_public_key,
            authority_secret_key,
            cert_validity_sec,
            coinbase_reward_script,
            supported_extensions,
            required_extensions,
        }
    }

    /// Constructs a complete [`JDSConfig`] from a partial config and Pool settings.
    ///
    /// This is used when JDS is embedded in Pool. The partial config contains JDS-specific
    /// fields from the `[jds]` TOML section, while shared fields are inherited from Pool.
    #[allow(clippy::too_many_arguments)]
    pub fn from_partial(
        partial: JDSPartialConfig,
        engine_config: JobValidationEngineConfig,
        authority_public_key: Secp256k1PublicKey,
        authority_secret_key: Secp256k1SecretKey,
        cert_validity_sec: u64,
        coinbase_reward_script: CoinbaseRewardScript,
    ) -> Self {
        Self {
            listen_address: partial.listen_address,
            engine_config,
            authority_public_key,
            authority_secret_key,
            cert_validity_sec,
            coinbase_reward_script,
            supported_extensions: partial.supported_extensions,
            required_extensions: partial.required_extensions,
        }
    }

    /// Address the JDS downstream server listens on (Noise-encrypted JDP).
    pub fn listen_address(&self) -> &SocketAddr {
        &self.listen_address
    }

    /// Which backend engine (and its connection params) is used for job validation.
    pub fn engine_config(&self) -> &JobValidationEngineConfig {
        &self.engine_config
    }

    /// Authority public key used for the Noise handshake with downstreams.
    pub fn authority_public_key(&self) -> &Secp256k1PublicKey {
        &self.authority_public_key
    }

    /// Authority secret key used for the Noise handshake with downstreams.
    pub fn authority_secret_key(&self) -> &Secp256k1SecretKey {
        &self.authority_secret_key
    }

    /// Validity period (seconds) for the Noise certificate.
    pub fn cert_validity_sec(&self) -> u64 {
        self.cert_validity_sec
    }

    /// Script included in `AllocateMiningJobTokenSuccess.coinbase_outputs`.
    pub fn coinbase_reward_script(&self) -> &CoinbaseRewardScript {
        &self.coinbase_reward_script
    }

    /// SV2 extension types that JDS supports.
    pub fn supported_extensions(&self) -> &[u16] {
        &self.supported_extensions
    }

    /// SV2 extension types that JDS requires from downstreams.
    pub fn required_extensions(&self) -> &[u16] {
        &self.required_extensions
    }
}

/// Configuration for the engine that JDS will use to fetch data about the Bitcoin Network.
#[derive(Clone, Debug, serde::Deserialize)]
pub enum JobValidationEngineConfig {
    /// Bitcoin Core over IPC
    BitcoinCoreIPC {
        /// Network for determining socket path subdirectory.
        network: BitcoinNetwork,
        /// Custom Bitcoin data directory. Uses OS default if not set.
        #[serde(default, deserialize_with = "opt_path_from_toml")]
        data_dir: Option<PathBuf>,
    },
    // todo e.g.: RPC, ZMQ
}
