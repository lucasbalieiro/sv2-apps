use stratum_apps::stratum_core::sv1_api::{json_rpc, Message};

pub(super) mod channel;
mod difficulty_manager;
pub mod downstream_message_handler;
pub mod sv1_server;

use tracing::warn;

/// Delimiter used to separate original job ID from keepalive mutation counter.
/// Format: `{original_job_id}#{counter}`
const KEEPALIVE_JOB_ID_DELIMITER: char = '#';

/// Check if Sv1 message is mining.authorize
fn is_mining_authorize(msg: &Message) -> bool {
    if let json_rpc::Message::StandardRequest(r) = &msg {
        r.method == "mining.authorize"
    } else {
        false
    }
}

/// Truncates a string to [`MAX_USER_IDENTITY_BYTES`], respecting UTF-8 character boundaries.
///
/// If the input string exceeds the limit, it is truncated at the last valid UTF-8 character
/// boundary before or at [`MAX_USER_IDENTITY_BYTES`] and a warning is logged.
fn tlv_compatible_username(s: &str) -> &str {
    const MAX_USER_IDENTITY_BYTES: usize = 32;
    let len = s.len();

    if len <= MAX_USER_IDENTITY_BYTES {
        return s;
    }
    // Find the last valid UTF-8 char boundary at or before MAX_USER_IDENTITY_BYTES
    let mut end = MAX_USER_IDENTITY_BYTES;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    let truncated = &s[..end];
    warn!(
        "Username '{}' exceeds {} bytes ({} bytes), truncating to '{}'. \
         Consider using a shorter username for full visibility on the pool dashboard.",
        s, MAX_USER_IDENTITY_BYTES, len, truncated
    );
    truncated
}
