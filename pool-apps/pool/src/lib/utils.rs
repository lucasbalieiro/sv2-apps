use std::net::SocketAddr;
use stratum_apps::{
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
/// - `Solo`: Full reward goes to the miner's specified payout address.
///   Pattern: `sri/solo/<payout_address>/<worker_name>`
/// - `Donate`: Partial donation to pool, remainder to miner.
///   Pattern: `sri/donate/<percentage>/<payout_address>/<worker_name>`
///   (percentage is 0-100, representing pool's share)
/// - `Pool`: Full reward goes to the pool.
pub enum PayoutMode {
    /// Solo mode: miner receives full block reward.
    Solo(String),
    /// Donate mode: pool receives specified percentage, miner gets remainder.
    Donate(u8, String),
    /// Pool mode: full reward goes to the pool.
    Pool,
}

/// Validates the user_identity string and returns the corresponding [`PayoutMode`].
///
/// The user_identity format is: `sri/<mode>/...` where mode is either `solo` or `donate`.
///
/// # Arguments
/// * `user_identity` - The user identity string from the mining client
/// * `solo_mining_mode` - Whether solo mining mode is enabled in the pool config
///
/// # Returns
/// * `Ok(PayoutMode)` - The parsed payout mode
/// * `Err(())` - Invalid user_identity format
///
/// # Patterns
/// - `sri/donate/worker_name` -> Pool mode (full donation to pool)
/// - `sri/donate/<percentage>/<payout_address>/<worker_name>` -> Donate mode (partial donation)
/// - `sri/solo/<payout_address>/<worker_name>` -> Solo mode (full reward to miner)
pub fn validate_user_identity(
    user_identity: &str,
    solo_mining_mode: bool,
) -> Result<PayoutMode, ()> {
    if !solo_mining_mode {
        return Ok(PayoutMode::Pool);
    }

    let parts: Vec<&str> = user_identity.split('/').collect();

    if parts.is_empty() || parts[0] != "sri" || parts.len() < 2 {
        return Err(());
    }

    match parts[1] {
        "solo" => {
            if parts.len() >= 3 && !parts[2].is_empty() {
                Ok(PayoutMode::Solo(parts[2].to_string()))
            } else {
                Err(())
            }
        }
        "donate" => {
            if parts.len() == 3 {
                Ok(PayoutMode::Pool)
            } else if parts.len() >= 5 && !parts[3].is_empty() {
                let percentage: u8 = parts[2].parse().map_err(|_| ())?;
                if percentage > 100 {
                    return Err(());
                }
                Ok(PayoutMode::Donate(percentage, parts[3].to_string()))
            } else {
                Err(())
            }
        }
        _ => Err(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_solo_mining_mode_disabled() {
        assert!(matches!(
            validate_user_identity("sri/donate/worker", false),
            Ok(PayoutMode::Pool)
        ));
        assert!(matches!(
            validate_user_identity("sri/solo/bc1q.../worker", false),
            Ok(PayoutMode::Pool)
        ));
        assert!(matches!(
            validate_user_identity("anything", false),
            Ok(PayoutMode::Pool)
        ));
        assert!(matches!(
            validate_user_identity("", false),
            Ok(PayoutMode::Pool)
        ));
    }

    #[test]
    fn test_valid_pool_donate() {
        assert!(matches!(
            validate_user_identity("sri/donate/worker", true),
            Ok(PayoutMode::Pool)
        ));
        assert!(matches!(
            validate_user_identity("sri/donate/myworker", true),
            Ok(PayoutMode::Pool)
        ));
    }

    #[test]
    fn test_valid_solo() {
        assert!(matches!(
            validate_user_identity("sri/solo/bc1qxyz/worker", true),
            Ok(PayoutMode::Solo(addr)) if addr == "bc1qxyz"
        ));
        assert!(matches!(
            validate_user_identity("sri/solo/tb1qxyz/worker/subworker", true),
            Ok(PayoutMode::Solo(addr)) if addr == "tb1qxyz"
        ));
    }

    #[test]
    fn test_valid_donate_with_percentage() {
        assert!(matches!(
            validate_user_identity("sri/donate/50/bc1qxyz/worker", true),
            Ok(PayoutMode::Donate(50, addr)) if addr == "bc1qxyz"
        ));
        assert!(matches!(
            validate_user_identity("sri/donate/0/bc1qxyz/worker", true),
            Ok(PayoutMode::Donate(0, addr)) if addr == "bc1qxyz"
        ));
        assert!(matches!(
            validate_user_identity("sri/donate/100/bc1qxyz/worker", true),
            Ok(PayoutMode::Donate(100, addr)) if addr == "bc1qxyz"
        ));
    }

    #[test]
    fn test_invalid_patterns() {
        assert!(validate_user_identity("sri/invalid/worker", true).is_err());
        assert!(validate_user_identity("sri/solo", true).is_err());
        assert!(validate_user_identity("sri/solo/", true).is_err());
        assert!(validate_user_identity("sri/donate/abc/addr/worker", true).is_err());
        assert!(validate_user_identity("sri/donate/101/addr/worker", true).is_err());
        assert!(validate_user_identity("sri/donate/50/addr", true).is_err());
        assert!(validate_user_identity("other/donate/worker", true).is_err());
        assert!(validate_user_identity("sri", true).is_err());
        assert!(validate_user_identity("", true).is_err());
    }
}
