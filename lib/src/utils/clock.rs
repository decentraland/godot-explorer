//! Clock for the `x-identity-timestamp` on signed requests.
//!
//! Services verify that timestamp against their own clock inside a ±60 s window, so a device
//! running a minute fast gets rejections that read as signature failures. App Review hit exactly
//! that: an iPad ~60 s ahead had its credits `quote` refused with "Signature timestamp is too far
//! in the future", and the purchase never reached StoreKit.
//!
//! We learn an offset over SNTP and add it to the device clock when signing. If nothing answers
//! the offset stays 0 and signing behaves exactly as it did before.

use std::sync::atomic::{AtomicI64, AtomicU8, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tokio::net::UdpSocket;
use tokio::runtime::Handle;
use tokio::time::timeout;

/// Tried in order; the first valid reply wins.
const NTP_SERVERS: [&str; 4] = [
    "time.cloudflare.com:123",
    "time.google.com:123",
    "time.apple.com:123",
    "pool.ntp.org:123",
];

const QUERY_TIMEOUT: Duration = Duration::from_secs(3);
const REFRESH_INTERVAL: Duration = Duration::from_secs(15 * 60);
const RETRY_INTERVAL: Duration = Duration::from_secs(60);
/// Past this an offset is garbage rather than a real clock fault: a device further off than this
/// cannot validate a TLS certificate, so nothing else in the app would work either. Comfortably
/// clears the largest timezone offset (UTC+14) applied by mistake.
const MAX_PLAUSIBLE_OFFSET_MS: i64 = 24 * 60 * 60 * 1000;
/// Seconds between the NTP epoch (1900-01-01) and the Unix epoch.
const NTP_UNIX_DELTA: i64 = 2_208_988_800;

pub const SYNC_PENDING: u8 = 0;
pub const SYNC_OK: u8 = 1;
pub const SYNC_FAILED: u8 = 2;

static OFFSET_MS: AtomicI64 = AtomicI64::new(0);
static STATE: AtomicU8 = AtomicU8::new(SYNC_PENDING);

/// Device clock corrected by the learned offset, in milliseconds since the Unix epoch.
/// Use this for any timestamp a server verifies against its own clock.
pub fn unix_time_ms() -> u128 {
    device_unix_time_ms()
        .saturating_add(OFFSET_MS.load(Ordering::Relaxed))
        .max(0) as u128
}

/// How far the device clock is behind real time, in milliseconds. Positive means the device is
/// slow. 0 while unsynced.
pub fn offset_ms() -> i64 {
    OFFSET_MS.load(Ordering::Relaxed)
}

pub fn sync_state() -> u8 {
    STATE.load(Ordering::Relaxed)
}

/// Initial sync plus a periodic refresh. Failures back off to `RETRY_INTERVAL` and keep the last
/// known offset.
pub fn spawn_background_sync(handle: &Handle) {
    handle.spawn(async {
        loop {
            let wait = match sync_now().await {
                Ok(offset) => {
                    tracing::info!("signing clock synced, offset {offset} ms");
                    REFRESH_INTERVAL
                }
                Err(e) => {
                    tracing::warn!("signing clock sync failed, using the device clock: {e}");
                    RETRY_INTERVAL
                }
            };
            tokio::time::sleep(wait).await;
        }
    });
}

pub async fn sync_now() -> Result<i64, String> {
    let mut last_error = "no servers configured".to_owned();
    for server in NTP_SERVERS {
        let result = timeout(QUERY_TIMEOUT, query(server))
            .await
            .unwrap_or_else(|_| Err("timeout".to_owned()));
        match result {
            Ok(offset) => {
                OFFSET_MS.store(offset, Ordering::Relaxed);
                STATE.store(SYNC_OK, Ordering::Relaxed);
                return Ok(offset);
            }
            Err(e) => {
                tracing::debug!("ntp query to {server} failed: {e}");
                last_error = format!("{server}: {e}");
            }
        }
    }
    // A later failure must not throw away an offset we already trust.
    let _ = STATE.compare_exchange(
        SYNC_PENDING,
        SYNC_FAILED,
        Ordering::Relaxed,
        Ordering::Relaxed,
    );
    Err(last_error)
}

async fn query(server: &str) -> Result<i64, String> {
    let socket = UdpSocket::bind("0.0.0.0:0")
        .await
        .map_err(|e| format!("bind: {e}"))?;
    socket
        .connect(server)
        .await
        .map_err(|e| format!("connect: {e}"))?;

    let mut request = [0u8; 48];
    request[0] = 0x23; // LI 0, version 4, mode 3 (client)
    let t1 = device_unix_time_ms();
    write_ntp_timestamp(&mut request[40..48], t1);
    socket
        .send(&request)
        .await
        .map_err(|e| format!("send: {e}"))?;

    let mut response = [0u8; 48];
    let read = socket
        .recv(&mut response)
        .await
        .map_err(|e| format!("recv: {e}"))?;
    let t4 = device_unix_time_ms();

    if read < 48 {
        return Err(format!("short reply, {read} bytes"));
    }
    if response[0] & 0b111 != 4 {
        return Err("not a server reply".to_owned());
    }
    if response[0] >> 6 == 3 {
        return Err("server clock unsynchronised".to_owned());
    }
    if response[1] == 0 {
        return Err("kiss-o'-death".to_owned());
    }
    // The reply has to echo the transmit timestamp we sent (RFC 4330 §5). Without this any
    // datagram arriving on the ephemeral port is accepted, so a stale duplicate or an off-path
    // spoofer only has to guess the port. Compared as raw bytes: the encoder floors and the
    // decoder rounds, so the server echoes these eight bytes verbatim but not the decoded value.
    if response[24..32] != request[40..48] {
        return Err("originate timestamp mismatch".to_owned());
    }

    let t2 = read_ntp_timestamp(&response[32..40]);
    let t3 = read_ntp_timestamp(&response[40..48]);
    if t2 == 0 || t3 == 0 {
        return Err("empty timestamp".to_owned());
    }

    // One bad reply must not be able to shift every signed request in the app. Rejecting falls
    // through to the next server.
    let offset = offset_from(t1, t2, t3, t4);
    if offset.abs() > MAX_PLAUSIBLE_OFFSET_MS {
        return Err(format!("implausible offset {offset} ms"));
    }
    Ok(offset)
}

/// Clock offset from the four timestamps of a round trip (RFC 4330 §5).
fn offset_from(t1: i64, t2: i64, t3: i64, t4: i64) -> i64 {
    ((t2 - t1) + (t3 - t4)) / 2
}

fn device_unix_time_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn read_ntp_timestamp(bytes: &[u8]) -> i64 {
    let secs = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as i64;
    let frac = u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]) as i64;
    if secs == 0 && frac == 0 {
        return 0;
    }
    // Round rather than truncate: the encoder already floors, and two floors in a row cost a
    // whole millisecond.
    (secs - NTP_UNIX_DELTA) * 1000 + (((frac * 1000) + (1 << 31)) >> 32)
}

fn write_ntp_timestamp(bytes: &mut [u8], unix_ms: i64) {
    let secs = (unix_ms / 1000 + NTP_UNIX_DELTA) as u32;
    let frac = (((unix_ms % 1000) << 32) / 1000) as u32;
    bytes[0..4].copy_from_slice(&secs.to_be_bytes());
    bytes[4..8].copy_from_slice(&frac.to_be_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The window services enforce on `x-identity-timestamp`.
    const WINDOW_MS: i64 = 60_000;

    /// A device far enough ahead to be rejected has to end up inside the window once corrected,
    /// with the encoding round-tripping and the round trip cancelling out.
    #[test]
    fn brings_a_skewed_device_back_inside_the_signing_window() {
        // Device 90 s ahead, 40 ms round trip, 1 ms of server processing.
        let skew = 90_000i64;
        let server_t2 = 1_788_749_031_686i64;
        let server_t3 = server_t2 + 1;
        let t1 = server_t2 + skew - 20;
        let t4 = server_t3 + skew + 20;

        let mut buf = [0u8; 8];
        write_ntp_timestamp(&mut buf, server_t2);
        assert_eq!(read_ntp_timestamp(&buf), server_t2);

        let offset = offset_from(t1, server_t2, server_t3, t4);
        assert!(
            (offset + skew).abs() <= 1,
            "offset {offset} should undo the {skew} ms skew"
        );
        assert!(
            (t1 - server_t2).abs() >= WINDOW_MS,
            "the device clock alone would be rejected"
        );
        assert!(
            ((t1 + offset) - server_t2).abs() < WINDOW_MS,
            "the corrected clock has to be accepted"
        );
    }
}
