use std::collections::HashMap;
use std::time::{Duration, Instant};

use godot::prelude::*;

use crate::godot_classes::dcl_android_plugin::DclAndroidPlugin;

use super::data_definition::{SegmentEvent, SegmentEventInstallAttribution};

/// Install attribution from the two Android mechanisms that can carry a campaign, resolved
/// into one event (issue #2670). Both are needed because they cover disjoint traffic: GA4F
/// deferred deep links are the only path that reaches Google Ads installs (which arrive with
/// a bare `gclid` and nothing of ours), the Play referrer preserves the query string of an
/// owned or organic link.
///
/// GA4F wins only when its link carries a token — most do not, and letting those beat a
/// referrer that has one would lose the campaign for good. The loser still contributes its
/// own fields. Fire-once-per-install is persisted by GDScript via a config flag.
pub struct InstallAttribution {
    done: bool,
    /// GA4F cannot say "there will never be a link" — an organic install simply never gets
    /// one, so its read stays pending forever. Past this deadline the referrer answers alone.
    ga4f_deadline: Instant,
    /// The referrer can hang too (its status only changes from a Play callback), and
    /// attribution runs once per install, so without a second bound the event is never sent.
    overall_deadline: Instant,
    started: Instant,
}

/// How long to let GA4F answer before falling back to the referrer. It writes within a second
/// or two of Firebase init; this leaves room for a cold start on a slow device.
const GA4F_WAIT: Duration = Duration::from_secs(10);

/// Hard bound on the whole resolution. Past this the event is emitted with whatever is
/// known, so a source that never answers cannot swallow the install entirely.
const OVERALL_WAIT: Duration = Duration::from_secs(25);

// Which mechanism reported. Independent of whether a token came back — `campaign_token`
// answers that, and tying them together would label two identical ad installs differently.
const SOURCE_GA4F: &str = "ga4f_deferred_deeplink";
const SOURCE_REFERRER: &str = "play_install_referrer";
const SOURCE_NONE: &str = "none";

/// The query param carrying the campaign token, in both sources. Not `utm_content`: that
/// already carries a platform label on our owned links, so overloading it would make every
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

    /// Whether resolution has finished, event or not. Callers must drop the tracker on this
    /// rather than on `poll()` returning an event: two settle paths have nothing to report,
    /// and treating those as "still running" strands every consumer waiting on it.
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

        // The referrer's fields are only meaningful once it says "ok" — a pending dict carries
        // just a status, and reading through it would ship zeros as data. GA4F is kept whenever
        // it answered, token or not: the link and its click time are worth reporting.
        let ga4f_ok = ga4f
            .as_ref()
            .filter(|(_, status)| status == "ok")
            .map(|(dict, _)| dict);
        let referrer_ok = referrer
            .as_ref()
            .filter(|(_, status)| status == "ok")
            .map(|(dict, _)| dict);

        // GA4F decides the destination only when its link actually carries a token. Winning on
        // status alone would discard a valid referrer token whenever the ad group's link has
        // none, and the campaign is lost for good: attribution runs once per install.
        if let Some(dict) = ga4f_ok {
            if let Some(token) = extract_token(&get_string(dict, "deeplink")) {
                self.done = true;
                return Some(self.build(Some(dict), referrer_ok, Some(token), SOURCE_GA4F));
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

        // The referrer decides from here. It gets its own deadline: its status only changes
        // from a Play service callback, and a binding that never calls back would otherwise
        // swallow the install entirely, unretryably.
        let referrer_status = referrer.as_ref().map(|(_, s)| s.as_str()).unwrap_or("none");
        if referrer_status == "pending" && now < self.overall_deadline {
            return None;
        }

        self.done = true;

        if let Some(dict) = referrer_ok {
            let token = extract_token(&get_string(dict, "referrer"));
            // GA4F still names the source when it answered, even though the referrer supplied
            // the token: the label says which mechanism reported, and GA4F is the more
            // specific one. Deciding it on token presence instead would put two identical ad
            // installs in different buckets depending on whether Play happened to answer.
            let source = if ga4f_ok.is_some() {
                SOURCE_GA4F
            } else {
                SOURCE_REFERRER
            };
            return Some(self.build(ga4f_ok, Some(dict), token, source));
        }

        // The referrer never answered, or answered not_available/error (no Play Store, or the
        // service is down). Report what is known rather than dropping the install: on a device
        // where GA4F answered, that still carries the deep link.
        if referrer_status != "none" {
            tracing::warn!("[attribution] referrer unusable (status='{referrer_status}')");
        }
        if ga4f_ok.is_none() && referrer.is_none() {
            // Not on Android, or no plugin at all: there is genuinely nothing to report.
            return None;
        }
        // GA4F may still have answered with a link that carried no token. That is the GA4F
        // path, not "nothing answered" — reporting `none` would make it indistinguishable in
        // Segment from an install where neither source ever replied.
        let source = if ga4f_ok.is_some() {
            SOURCE_GA4F
        } else {
            SOURCE_NONE
        };
        Some(self.build(ga4f_ok, None, None, source))
    }

    /// Builds the event from whichever sources answered. Both dicts are optional and
    /// independent, and fields are only read from one that is present, so a source that never
    /// answered contributes nothing rather than zeros.
    fn build(
        &self,
        ga4f: Option<&VarDictionary>,
        referrer: Option<&VarDictionary>,
        token: Option<String>,
        source: &str,
    ) -> SegmentEvent {
        let referrer_string = referrer
            .map(|d| get_string(d, "referrer"))
            .unwrap_or_default();
        let utm = parse_utm_params(&referrer_string);
        let deeplink = ga4f.map(|d| get_string(d, "deeplink"));

        // Both sources report a click time in seconds, but they describe different clicks, so
        // this is read from the one `attribution_source` names and never falls back to the
        // other. A 0 here means that source reported no click time — which GA4F does whenever
        // its timestamp pref is missing — and is more honest than silently substituting a
        // different click. The install time only ever exists on the referrer.
        let click_timestamp = if source == SOURCE_GA4F {
            ga4f.map(|d| get_i64(d, "click_timestamp")).unwrap_or(0)
        } else {
            referrer.map(|d| get_i64(d, "click_timestamp")).unwrap_or(0)
        };

        tracing::debug!(
            "[attribution] settled: source={} token={:?} ga4f={} referrer={} after {:?}",
            source,
            token,
            ga4f.is_some(),
            referrer.is_some(),
            self.started.elapsed()
        );

        SegmentEvent::InstallAttribution(SegmentEventInstallAttribution {
            referrer: referrer_string,
            utm_source: utm.get("utm_source").cloned(),
            utm_medium: utm.get("utm_medium").cloned(),
            utm_campaign: utm.get("utm_campaign").cloned(),
            utm_content: utm.get("utm_content").cloned(),
            utm_term: utm.get("utm_term").cloned(),
            click_timestamp,
            install_timestamp: referrer
                .map(|d| get_i64(d, "install_timestamp"))
                .unwrap_or(0),
            google_play_instant: referrer
                .map(|d| get_bool(d, "google_play_instant"))
                .unwrap_or(false),
            attribution_source: source.to_string(),
            campaign_token: token,
            deferred_deep_link: deeplink,
            referrer_settled: referrer.is_some(),
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

        // A Google Ads install: a bare click id, nothing of ours. This is the majority shape,
        // and it must not produce a token.
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
