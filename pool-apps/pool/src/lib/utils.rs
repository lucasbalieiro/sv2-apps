use std::net::SocketAddr;
use stratum_apps::{
    config_helpers::CoinbaseRewardScript,
    stratum_core::bitcoin::{Amount, TxOut},
    stratum_core::{
        binary_sv2::Str0255,
        common_messages_sv2::{Protocol, SetupConnection},
        mining_sv2::CloseChannel,
    },
    utils::types::ChannelId,
};

use crate::error::PoolErrorKind;

/// Constructs a `SetupConnection` message for the mining protocol.
#[allow(clippy::result_large_err)]
pub fn get_setup_connection_message(
    min_version: u16,
    max_version: u16,
    address: &SocketAddr,
) -> Result<SetupConnection<'static>, PoolErrorKind> {
    let endpoint_host = address.ip().to_string().into_bytes().try_into()?;
    let vendor = String::new().try_into()?;
    let hardware_version = String::new().try_into()?;
    let firmware = String::new().try_into()?;
    let device_id = String::new().try_into()?;
    let flags = 0b0000_0000_0000_0000_0000_0000_0000_0110;
    Ok(SetupConnection {
        protocol: Protocol::MiningProtocol,
        min_version,
        max_version,
        flags,
        endpoint_host,
        endpoint_port: address.port(),
        vendor,
        hardware_version,
        firmware,
        device_id,
    })
}

/// Constructs a `SetupConnection` message for the Template Provider (TP).
#[allow(clippy::result_large_err)]
pub fn get_setup_connection_message_tp(
    address: SocketAddr,
) -> Result<SetupConnection<'static>, PoolErrorKind> {
    let endpoint_host = address.ip().to_string().into_bytes().try_into()?;
    let vendor = String::new().try_into()?;
    let hardware_version = String::new().try_into()?;
    let firmware = String::new().try_into()?;
    let device_id = String::new().try_into()?;
    Ok(SetupConnection {
        protocol: Protocol::TemplateDistributionProtocol,
        min_version: 2,
        max_version: 2,
        flags: 0b0000_0000_0000_0000_0000_0000_0000_0000,
        endpoint_host,
        endpoint_port: address.port(),
        vendor,
        hardware_version,
        firmware,
        device_id,
    })
}

/// Creates a [`CloseChannel`] message for the given channel ID and reason.
///
/// The `msg` is converted into a [`Str0255`] reason code.  
/// If conversion fails, this function will panic.
pub(crate) fn create_close_channel_msg(channel_id: ChannelId, msg: &str) -> CloseChannel<'_> {
    CloseChannel {
        channel_id,
        reason_code: Str0255::try_from(msg.to_string()).expect("Could not convert message."),
    }
}

/// Represents the payout mode for a mining connection.
///
/// This determines how the coinbase reward is distributed:
/// - `Solo`: Full reward goes to the miner's specified payout address. Pattern:
///   `sri/solo/<payout_address>/<worker_name>` or plain Bitcoin address
/// - `Donate`: Partial donation to pool, remainder to miner. Pattern:
///   `sri/donate/<percentage>/<payout_address>/<worker_name>` (percentage is 0-100, representing
///   pool's portion)
/// - `FullDonation`: Full reward goes to the pool.
#[derive(Debug)]
pub enum PayoutMode {
    /// Solo mode: miner receives full block reward.
    Solo(CoinbaseRewardScript),
    /// Donate mode: pool receives specified percentage, miner gets remainder.
    Donate(u8, CoinbaseRewardScript),
    /// Full donation mode: full reward goes to the pool.
    FullDonation,
}

impl PayoutMode {
    pub fn coinbase_outputs(
        &self,
        total_value: u64,
        pool_script: &CoinbaseRewardScript,
    ) -> Vec<TxOut> {
        match self {
            PayoutMode::Solo(coinbase_script) => {
                vec![TxOut {
                    value: Amount::from_sat(total_value),
                    script_pubkey: coinbase_script.script_pubkey(),
                }]
            }

            PayoutMode::Donate(percentage, miner_script) => {
                let pool_value = ((total_value as u128 * *percentage as u128) / 100) as u64;
                let miner_value = total_value.saturating_sub(pool_value);

                vec![
                    TxOut {
                        value: Amount::from_sat(pool_value),
                        script_pubkey: pool_script.script_pubkey(),
                    },
                    TxOut {
                        value: Amount::from_sat(miner_value),
                        script_pubkey: miner_script.script_pubkey(),
                    },
                ]
            }

            PayoutMode::FullDonation => {
                vec![TxOut {
                    value: Amount::from_sat(total_value),
                    script_pubkey: pool_script.script_pubkey(),
                }]
            }
        }
    }
}

/// Represents a parsed user_identity pattern.
#[derive(Debug)]
enum UserIdentityPattern {
    /// sri/donate/<worker_name> -> Pool mode
    SriFullDonation,
    /// sri/donate/<percentage>/<payout_address>/<worker_name> -> Donate mode
    SriPartialDonation(u8, CoinbaseRewardScript),
    /// sri/solo/<payout_address>/<worker_name> -> Solo mode
    /// <valid_bitcoin_address> -> Solo mode
    SriSolo(CoinbaseRewardScript),
    /// Unknown pattern -> In this case we assume that the pattern is vali but we do NOT take
    /// action based on it.
    Unknown,
    /// Invalid Pattern -> In this case the user tried to pass something that could be understood
    /// as a solo mining request, btu it was malformed
    Invalid,
}

/// Parses the user_identity string into a [`UserIdentityPattern`].
///
/// # Patterns
/// - Plain Bitcoin address (e.g., "bc1q...") -> Solo mode
/// - `sri/donate/worker_name` -> Full Donation mode
/// - `sri/donate/percentage/payout_address/worker_name` -> Partial Donation mode
/// - `sri/solo/payout_address/worker_name` -> Solo mode
fn parse_user_identity(user_identity: &str) -> UserIdentityPattern {
    if user_identity.is_empty() {
        return UserIdentityPattern::Unknown;
    }

    // Handle plain address or address.worker
    let addr = user_identity
        .split_once('.')
        .map(|(addr, _)| addr)
        .unwrap_or(user_identity);

    let descriptor = format!("addr({addr})");
    if let Ok(script) = CoinbaseRewardScript::from_descriptor(&descriptor) {
        return UserIdentityPattern::SriSolo(script);
    }

    // Parse sri/... patterns
    let mut parts = user_identity.split('/');

    match (parts.next(), parts.next(), parts.next(), parts.next()) {
        // sri/solo/<payout_address>/<worker_name>
        (Some("sri"), Some("solo"), Some(payout_address), _) => {
            let descriptor = format!("addr({payout_address})");
            if let Ok(script) = CoinbaseRewardScript::from_descriptor(&descriptor) {
                UserIdentityPattern::SriSolo(script)
            } else {
                UserIdentityPattern::Invalid
            }
        }

        // sri/donate OR sri/donate/<worker_name>
        (Some("sri"), Some("donate"), None, _) | (Some("sri"), Some("donate"), Some(_), None) => {
            UserIdentityPattern::SriFullDonation
        }

        // sri/donate/<percentage>/<payout_address>/<worker_name>
        (Some("sri"), Some("donate"), Some(percentage), Some(payout_address)) => {
            let descriptor = format!("addr({payout_address})");

            match (
                percentage.parse::<u8>(),
                CoinbaseRewardScript::from_descriptor(&descriptor),
            ) {
                (Ok(p), Ok(script)) if p <= 100 => {
                    UserIdentityPattern::SriPartialDonation(p, script)
                }
                _ => UserIdentityPattern::Invalid,
            }
        }

        // sri/<unknown_mode>/...
        (Some("sri"), Some(_), _, _) => UserIdentityPattern::Invalid,

        // doesn't start with sri and not a valid address
        _ => UserIdentityPattern::Unknown,
    }
}
/// Validates the user_identity string and returns the corresponding [`PayoutMode`].
///
/// # Arguments
/// * `user_identity` - The user identity string from the mining client
///
/// # Returns
/// * `Ok(PayoutMode)` - The parsed payout mode
/// * `Err(PoolErrorKind)` - Invalid user_identity format
///
/// # Patterns
/// - Plain Bitcoin address (e.g., "bc1q...") -> Solo mode (full reward to miner)
/// - `sri/donate/worker_name` -> Pool mode (full donation to pool)
/// - `sri/donate/<percentage>/<payout_address>/<worker_name>` -> Donate mode (partial donation)
/// - `sri/solo/<payout_address>/<worker_name>` -> Solo mode (full reward to miner)
#[allow(clippy::result_large_err)]
pub fn validate_user_identity(user_identity: &str) -> Result<PayoutMode, PoolErrorKind> {
    match parse_user_identity(user_identity) {
        UserIdentityPattern::SriSolo(script) => Ok(PayoutMode::Solo(script)),
        UserIdentityPattern::SriFullDonation => Ok(PayoutMode::FullDonation),
        UserIdentityPattern::SriPartialDonation(percentage, script) => {
            Ok(PayoutMode::Donate(percentage, script))
        }
        UserIdentityPattern::Unknown => Ok(PayoutMode::FullDonation),
        UserIdentityPattern::Invalid => Err(PoolErrorKind::PayoutModeError(
            "Invalid user_identity pattern for solo mining mode.".to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use stratum_apps::stratum_core::bitcoin::{
        params::{MAINNET, TESTNET4},
        Address,
    };

    use super::*;

    #[test]
    fn test_valid_pool_donate() {
        assert!(matches!(
            validate_user_identity("sri/donate/worker"),
            Ok(PayoutMode::FullDonation)
        ));
        assert!(matches!(
            validate_user_identity("sri/donate"),
            Ok(PayoutMode::FullDonation)
        ));
    }

    #[test]
    fn test_valid_solo() {
        // Valid Bitcoin addresses (testnet and mainnet)
        let valid_testnet_addr = "tb1qa0sm0hxzj0x25rh8gw5xlzwlsfvvyz8u96w3p8";
        let valid_mainnet_addr = "bc1qtzqxqaxyy6lda2fhdtp5dp0v56vlf6g0tljy2x";

        assert!(matches!(
            validate_user_identity(&format!("sri/solo/{}/worker", valid_testnet_addr)),
                Ok(PayoutMode::Solo(script)) if Address::from_script(script.script_pubkey().as_script(), TESTNET4.clone()).unwrap().to_string() == valid_testnet_addr
        ));
        assert!(matches!(
            validate_user_identity(&format!("sri/solo/{}/worker/subworker", valid_mainnet_addr)),
            Ok(PayoutMode::Solo(script)) if Address::from_script(script.script_pubkey().as_script(), MAINNET.clone()).unwrap().to_string()== valid_mainnet_addr
        ));
        assert!(matches!(
            validate_user_identity(valid_mainnet_addr),
            Ok(PayoutMode::Solo(script)) if Address::from_script(script.script_pubkey().as_script(), MAINNET.clone()).unwrap().to_string() == valid_mainnet_addr
        ));

        assert!(matches!(
                validate_user_identity(valid_testnet_addr), 
                Ok(PayoutMode::Solo(script)) if Address::from_script(script.script_pubkey().as_script(), TESTNET4.clone()).unwrap().to_string() == valid_testnet_addr))
    }

    #[test]
    fn test_valid_solo_with_worker_suffix() {
        let valid_mainnet_addr = "bc1qtzqxqaxyy6lda2fhdtp5dp0v56vlf6g0tljy2x";

        assert!(matches!(
            validate_user_identity(&format!("{}.worker1", valid_mainnet_addr)),
            Ok(PayoutMode::Solo(script)) if Address::from_script(script.script_pubkey().as_script(), MAINNET.clone()).unwrap().to_string()== valid_mainnet_addr
        ));
        assert!(matches!(
            validate_user_identity(&format!("{}.worker1.subworker", valid_mainnet_addr)),
            Ok(PayoutMode::Solo(script)) if Address::from_script(script.script_pubkey().as_script(), MAINNET.clone()).unwrap().to_string() == valid_mainnet_addr
        ));
    }

    #[test]
    fn test_invalid_address_with_suffix() {
        assert!(matches!(
            validate_user_identity("invalid_address.worker"),
            Ok(PayoutMode::FullDonation)
        ));
    }

    #[test]
    fn test_valid_donate_with_percentage() {
        let valid_testnet_addr = "tb1qa0sm0hxzj0x25rh8gw5xlzwlsfvvyz8u96w3p8";

        assert!(matches!(
            validate_user_identity(&format!("sri/donate/50/{}/worker", valid_testnet_addr)).unwrap(),
            PayoutMode::Donate(50, script) if Address::from_script(script.script_pubkey().as_script(), TESTNET4.clone()).unwrap().to_string() == valid_testnet_addr
        ));

        assert!(matches!(
            validate_user_identity(&format!("sri/donate/50/{}/worker/subworker", valid_testnet_addr)).unwrap(),
            PayoutMode::Donate(50, script) if Address::from_script(script.script_pubkey().as_script(), TESTNET4.clone()).unwrap().to_string() == valid_testnet_addr
        ));

        assert!(matches!(
            validate_user_identity(&format!("sri/donate/0/{}/worker", valid_testnet_addr)).unwrap(),
            PayoutMode::Donate(0, script) if Address::from_script(script.script_pubkey().as_script(), TESTNET4.clone()).unwrap().to_string() == valid_testnet_addr
        ));

        assert!(matches!(
            validate_user_identity(&format!("sri/donate/100/{}/worker", valid_testnet_addr)).unwrap(),
            PayoutMode::Donate(100, script) if Address::from_script(script.script_pubkey().as_script(), TESTNET4.clone()).unwrap().to_string() == valid_testnet_addr
        ));
    }

    #[test]
    fn test_invalid_patterns() {
        let valid_testnet_addr = "tb1qa0sm0hxzj0x25rh8gw5xlzwlsfvvyz8u96w3p8";

        assert!(validate_user_identity("sri/invalid/worker").is_err());
        assert!(validate_user_identity("sri/solo").is_err());
        assert!(validate_user_identity("sri/solo/random_thing_here/worker").is_err());
        assert!(validate_user_identity("sri/solo/").is_err());
        assert!(matches!(
            validate_user_identity("sri/donate/abc/addr/worker"),
            Err(PoolErrorKind::PayoutModeError(_))
        ));
        assert!(matches!(
            validate_user_identity("sri/donate/101/addr/worker"),
            Err(PoolErrorKind::PayoutModeError(_))
        ));

        assert!(matches!(
            validate_user_identity(&format!("sri/donate/50/{}", valid_testnet_addr)).unwrap(),
            PayoutMode::Donate(50, script) if Address::from_script(script.script_pubkey().as_script(), TESTNET4.clone()).unwrap().to_string() == valid_testnet_addr
        ));
        assert!(matches!(
            validate_user_identity("other/donate/worker"),
            Ok(PayoutMode::FullDonation)
        ));
        assert!(matches!(
            validate_user_identity("sri/"),
            Err(PoolErrorKind::PayoutModeError(_))
        ));
        assert!(matches!(
            validate_user_identity(""),
            Ok(PayoutMode::FullDonation)
        ));
    }
}
