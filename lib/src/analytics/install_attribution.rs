use std::collections::HashMap;
use std::time::{Duration, Instant};

use godot::prelude::*;

use crate::godot_classes::dcl_android_plugin::DclAndroidPlugin;

use super::data_definition::{SegmentEvent, SegmentEventInstallAttribution};

/// Install attribution from the two Android mechanisms that can carry a campaign, resolved
/// into one event (issue #2670).
///
/// They cover disjoint traffic, which is why both are needed:
///
/// - **GA4F deferred deep link** — the only path that carries a destination for Google Ads
///   installs. Those arrive with a bare `gclid` in the Play referrer and nothing of ours
///   (87% of attributed installs over 30 days, measured on Segment), so tagging the landing
///   page does nothing for them.
/// - **Play Install Referrer** — preserves the query string of an owned/organic link, so a
///   token added there survives the install.
///
/// GA4F wins when both are present: an ad group's link is more specific than whatever
/// referrer Google happened to attach.
///
/// Persistence (fire-once-per-install) is handled by GDScript via a config flag.
pub struct InstallAttribution {
    done: bool,
    /// GA4F cannot say "there will never be a link" — an organic install simply never gets
    /// one, so its read stays pending forever. Past this deadline the referrer answers alone.
    ga4f_deadline: Instant,
    /// The referrer can hang too: its status only changes from a Play service callback, and a
    /// binding that never calls back leaves it pending forever. Without a second bound no
    /// event is ever emitted, and attribution runs once per install so it is never retried.
    overall_deadline: Instant,
    started: Instant,
}

/// How long to let GA4F answer before falling back to the referrer. The write happens on
/// first launch, within a second or two of the Firebase SDK initializing; this leaves room
/// for a cold start on a slow device without stalling the event indefinitely.
const GA4F_WAIT: Duration = Duration::from_secs(10);

/// Hard bound on the whole resolution. Past this the event is emitted with whatever is
/// known, so a source that never answers cannot swallow the install entirely.
const OVERALL_WAIT: Duration = Duration::from_secs(25);

const SOURCE_GA4F: &str = "ga4f_deferred_deeplink";
const SOURCE_REFERRER: &str = "play_install_referrer";
const SOURCE_NONE: &str = "none";

/// The query param carrying the campaign token, in both the deferred deep link and the
/// referrer. Deliberately not `utm_content`: that already carries a platform label on our
/// owned links (`utm_content=android`), and overloading it would make every existing
/// organic install look like a campaign whose token does not resolve.
const TOKEN_PARAM: &str = "c";

impl InstallAttribution {
    /// Create and start both fetches.
    pub fn start() -> Self {
        let _ = DclAndroidPlugin::get_install_referrer_internal();
        let _ = DclAndroidPlugin::get_deferred_deep_link_internal();
        let now = Instant::now();
        Self {
            done: false,
            ga4f_deadline: now + GA4F_WAIT,
            overall_deadline: now + OVERALL_WAIT,
            started: now,
        }
    }

    /// Whether resolution has finished, event or not.
    ///
    /// Callers must drop the tracker on this rather than on `poll()` returning an event: two
    /// settle paths legitimately have nothing to report, and treating those as "still
    /// running" strands every consumer waiting on it.
    pub fn is_done(&self) -> bool {
        self.done
    }

    /// Poll both sources. Returns a `SegmentEvent` once the attribution is settled, or
    /// `None` while still pending or after already completing.
    pub fn poll(&mut self) -> Option<SegmentEvent> {
        if self.done {
            return None;
        }

        let ga4f = read_status(DclAndroidPlugin::get_deferred_deep_link_internal());
        let referrer = read_status(DclAndroidPlugin::get_install_referrer_internal());

        // The referrer's own fields are only meaningful once it says "ok"; a pending dict
        // carries just a status, and reading through it would ship zeros as if they were data.
        let settled_referrer = referrer
            .as_ref()
            .filter(|(_, status)| status == "ok")
            .map(|(dict, _)| dict);

        // GA4F wins only when its link actually carries a token. Winning on status alone would
        // discard a valid referrer token whenever the ad group's link has none, and the
        // campaign is lost for good: attribution runs once per install.
        if let Some((ref dict, ref status)) = ga4f {
            if status == "ok" {
                if let Some(token) = extract_token(&get_string(dict, "deeplink")) {
                    self.done = true;
                    return Some(self.build_from_ga4f(dict, settled_referrer, token));
                }
            }
        }

        let ga4f_settled = match ga4f {
            // "pending" only means "not yet"; an organic install never resolves it.
            Some((_, ref status)) => status != "pending",
            None => true, // no plugin (non-Android, or the method is missing) — nothing to wait for
        };
        let now = Instant::now();
        if !ga4f_settled && now < self.ga4f_deadline {
            return None;
        }

        // Fall back to the referrer.
        let Some((dict, status)) = referrer else {
            // Not on Android, or no plugin: nothing to report at all.
            self.done = true;
            return None;
        };
        if status == "pending" {
            if now < self.overall_deadline {
                return None;
            }
            // The referrer never answered. Report the install anyway rather than losing it.
            self.done = true;
            tracing::warn!("[attribution] referrer never resolved, reporting without it");
            return Some(self.build_from_referrer(&VarDictionary::new()));
        }

        self.done = true;

        if status != "ok" {
            let error = get_string(&dict, "error");
            tracing::warn!("Install referrer not available: status='{status}' error='{error}'");
            return None;
        }

        Some(self.build_from_referrer(&dict))
    }

    fn build_from_ga4f(
        &self,
        ga4f: &VarDictionary,
        referrer: Option<&VarDictionary>,
        token: String,
    ) -> SegmentEvent {
        let deeplink = get_string(ga4f, "deeplink");

        // The referrer still carries the install timestamps and the raw string, which stay
        // worth reporting even when GA4F decided the destination.
        let referrer_string = referrer
            .map(|d| get_string(d, "referrer"))
            .unwrap_or_default();
        let utm = parse_utm_params(&referrer_string);

        tracing::debug!(
            "[attribution] GA4F deferred deep link won: token={:?} link='{}' after {:?}",
            token,
            deeplink,
            self.started.elapsed()
        );

        SegmentEvent::InstallAttribution(SegmentEventInstallAttribution {
            referrer: referrer_string,
            utm_source: utm.get("utm_source").cloned(),
            utm_medium: utm.get("utm_medium").cloned(),
            utm_campaign: utm.get("utm_campaign").cloned(),
            utm_content: utm.get("utm_content").cloned(),
            utm_term: utm.get("utm_term").cloned(),
            // GA4F reports its own click time; the install time only exists on the referrer.
            click_timestamp: get_i64(ga4f, "click_timestamp"),
            install_timestamp: referrer
                .map(|d| get_i64(d, "install_timestamp"))
                .unwrap_or(0),
            google_play_instant: referrer
                .map(|d| get_bool(d, "google_play_instant"))
                .unwrap_or(false),
            attribution_source: SOURCE_GA4F.to_string(),
            campaign_token: Some(token),
            deferred_deep_link: Some(deeplink),
        })
    }

    fn build_from_referrer(&self, dict: &VarDictionary) -> SegmentEvent {
        let referrer = get_string(dict, "referrer");
        let utm = parse_utm_params(&referrer);
        let token = extract_token(&referrer);

        // A referrer that carries no token of ours is still an attributed install worth
        // reporting — it is what a Google Ads `gclid` install looks like. Naming the source
        // `none` there keeps "we had no campaign" separate from "the referrer path won".
        let source = if token.is_some() {
            SOURCE_REFERRER
        } else {
            SOURCE_NONE
        };

        tracing::debug!(
            "[attribution] referrer path: source={} token={:?} referrer='{}'",
            source,
            token,
            referrer
        );

        SegmentEvent::InstallAttribution(SegmentEventInstallAttribution {
            referrer,
            utm_source: utm.get("utm_source").cloned(),
            utm_medium: utm.get("utm_medium").cloned(),
            utm_campaign: utm.get("utm_campaign").cloned(),
            utm_content: utm.get("utm_content").cloned(),
            utm_term: utm.get("utm_term").cloned(),
            click_timestamp: get_i64(dict, "click_timestamp"),
            install_timestamp: get_i64(dict, "install_timestamp"),
            google_play_instant: get_bool(dict, "google_play_instant"),
            attribution_source: source.to_string(),
            campaign_token: token,
            deferred_deep_link: None,
        })
    }
}

fn read_status(dict: Option<VarDictionary>) -> Option<(VarDictionary, String)> {
    let dict = dict?;
    let status = get_string(&dict, "status");
    Some((dict, status))
}

fn get_string(dict: &VarDictionary, key: &str) -> String {
    dict.get(key)
        .and_then(|v| v.try_to::<GString>().ok())
        .unwrap_or_default()
        .to_string()
}

fn get_i64(dict: &VarDictionary, key: &str) -> i64 {
    dict.get(key)
        .and_then(|v| v.try_to::<i64>().ok())
        .unwrap_or(0)
}

fn get_bool(dict: &VarDictionary, key: &str) -> bool {
    dict.get(key)
        .and_then(|v| v.try_to::<bool>().ok())
        .unwrap_or(false)
}

/// Pulls the campaign token out of a deep link or a referrer query string.
///
/// Both are handled by the same code on purpose: a referrer *is* a bare query string, and a
/// deep link is one with a scheme and path in front of it. Anything that is not a valid
/// token is dropped rather than forwarded, so a malformed link cannot become a lookup.
fn extract_token(input: &str) -> Option<String> {
    let query = input.split_once('?').map_or(input, |(_, q)| q);
    let query = query.split('#').next().unwrap_or(query);

    for pair in query.split('&') {
        let Some((key, value)) = pair.split_once('=') else {
            continue;
        };
        if key == TOKEN_PARAM {
            let decoded = percent_decode(value);
            if is_valid_token(&decoded) {
                return Some(decoded);
            }
            tracing::warn!("[attribution] ignoring malformed campaign token '{decoded}'");
        }
    }
    None
}

/// Mirrors the token rule the mobile-BFF enforces on save (kebab-case, max 64): a token that
/// cannot exist there is not worth a round trip.
fn is_valid_token(token: &str) -> bool {
    if token.is_empty() || token.len() > 64 {
        return false;
    }
    if token.starts_with('-') || token.ends_with('-') || token.contains("--") {
        return false;
    }
    token
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

fn parse_utm_params(referrer: &str) -> HashMap<String, String> {
    referrer
        .split('&')
        .filter_map(|pair| {
            let mut parts = pair.splitn(2, '=');
            let key = parts.next()?;
            let value = parts.next().unwrap_or("");
            if key.starts_with("utm_") {
                Some((key.to_string(), percent_decode(value)))
            } else {
                None
            }
        })
        .collect()
}

fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'%' && i + 2 < bytes.len() {
            let hi = (bytes[i + 1] as char).to_digit(16);
            let lo = (bytes[i + 2] as char).to_digit(16);
            if let (Some(h), Some(l)) = (hi, lo) {
                out.push((h * 16 + l) as u8);
                i += 3;
                continue;
            }
        }
        if b == b'+' {
            out.push(b' ');
        } else {
            out.push(b);
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::{extract_token, is_valid_token};

    // Whether a token is found decides whether an ad-driven install boots into the campaign
    // or gets the default FTUE, and the two sources hand this function very different
    // strings: a full deep link from GA4F, a bare query string from the referrer.
    #[test]
    fn extracts_the_token_from_both_attribution_shapes() {
        // GA4F: a real deep link, token among other params
        assert_eq!(
            extract_token("https://mobile.dclexplorer.com/open?c=summer2022&utm_source=ads"),
            Some("summer2022".to_string())
        );
        // GA4F with a fragment after the query
        assert_eq!(
            extract_token("https://mobile.dclexplorer.com/open?c=launch-26#frag"),
            Some("launch-26".to_string())
        );
        // Play referrer: a bare query string, no scheme or path
        assert_eq!(
            extract_token("utm_source=x&utm_medium=organic&c=winter-sale"),
            Some("winter-sale".to_string())
        );
        // Percent-encoded, as the referrer arrives from Play
        assert_eq!(
            extract_token("c%3Dnope&c=aesironline"),
            Some("aesironline".to_string())
        );

        // A Google Ads install: a bare click id, nothing of ours. This is 87% of attributed
        // installs, and it must not produce a token.
        assert_eq!(extract_token("gclid%3DCj0KCQjwp9vTBhCWARIsANaUrjv"), None);
        assert_eq!(extract_token("gbraid=0AAAAA_KoQHo8&gad_source=2"), None);
        // Our own organic tagging, before anyone adds a token to it
        assert_eq!(
            extract_token("utm_source=x&utm_medium=organic&utm_content=android"),
            None
        );
        assert_eq!(extract_token(""), None);

        // Present but unusable: dropped rather than forwarded, so a malformed link cannot
        // become a lookup against the BFF.
        assert_eq!(extract_token("c=Summer2022"), None);
        assert_eq!(extract_token("c=with%20space"), None);
    }

    #[test]
    fn accepts_only_tokens_the_backoffice_could_have_stored() {
        assert!(is_valid_token("summer2022"));
        assert!(is_valid_token("launch-26"));
        assert!(is_valid_token("a"));

        assert!(!is_valid_token(""));
        assert!(!is_valid_token("Summer")); // uppercase
        assert!(!is_valid_token("-lead")); // leading dash
        assert!(!is_valid_token("trail-")); // trailing dash
        assert!(!is_valid_token("double--dash"));
        assert!(!is_valid_token("under_score"));
        assert!(!is_valid_token(&"a".repeat(65)));
    }
}
