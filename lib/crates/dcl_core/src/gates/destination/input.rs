//! The input the shell hands `resolve_destination`, as a realm and an optional parcel.
//!
//! Accepted, in this order:
//! - a link: `decentraland://...`, or an `https` link on `decentraland.org`, `decentraland.zone`
//!   or `mobile.dclexplorer.com`, carrying `realm`, `position` / `location` or `preview`;
//! - coordinates alone: `x,y` (spaces and one pair of parentheses tolerated), in Genesis City;
//! - a realm, a `.dcl.eth` name or a URL, optionally followed by whitespace and `x,y`.

use url::Url;

use crate::{app::Parcel, deep_link};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Intent {
    /// Empty means the default realm.
    pub realm: String,
    pub parcel: Option<Parcel>,
}

const LINK_HOSTS: [&str; 3] = [
    "decentraland.org",
    "decentraland.zone",
    "mobile.dclexplorer.com",
];

fn is_link(input: &str) -> bool {
    let Ok(url) = Url::parse(input) else {
        return false;
    };
    match url.scheme() {
        "decentraland" => true,
        "http" | "https" => url.host_str().is_some_and(|h| LINK_HOSTS.contains(&h)),
        _ => false,
    }
}

fn coordinates(text: &str) -> Option<Parcel> {
    let text = text.trim();
    let text = text
        .strip_prefix('(')
        .and_then(|t| t.strip_suffix(')'))
        .unwrap_or(text);
    let (x, y) = text.split_once(',')?;
    Some(Parcel::new(x.trim().parse().ok()?, y.trim().parse().ok()?))
}

/// `None` when the input names nothing.
pub fn parse_input(input: &str) -> Option<Intent> {
    let input = input.trim();
    if input.is_empty() {
        return None;
    }
    if is_link(input) {
        let link = deep_link::parse_deep_link(input)?;
        let realm = if link.realm.is_empty() {
            link.preview
        } else {
            link.realm
        };
        let parcel = link.location.map(|(x, y)| Parcel::new(x, y));
        return (!realm.is_empty() || parcel.is_some()).then_some(Intent { realm, parcel });
    }
    if let Some(parcel) = coordinates(input) {
        return Some(Intent {
            realm: String::new(),
            parcel: Some(parcel),
        });
    }
    // The leftmost split whose tail is a coordinate pair, so "x, y" keeps its inner space.
    for (at, _) in input.match_indices(char::is_whitespace) {
        let (head, tail) = input.split_at(at);
        if let Some(parcel) = coordinates(tail) {
            let realm = head.trim();
            return looks_like_realm(realm).then(|| Intent {
                realm: realm.to_string(),
                parcel: Some(parcel),
            });
        }
    }
    looks_like_realm(input).then(|| Intent {
        realm: input.to_string(),
        parcel: None,
    })
}

/// A world name or a URL: one word with a dot or a scheme in it. Anything else would be sent to the network
/// as a host that cannot exist.
fn looks_like_realm(text: &str) -> bool {
    !text.contains(char::is_whitespace)
        && !text.contains(',')
        && (text.contains('.') || text.contains("://"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn intent(realm: &str, parcel: Option<(i32, i32)>) -> Option<Intent> {
        Some(Intent {
            realm: realm.into(),
            parcel: parcel.map(|(x, y)| Parcel::new(x, y)),
        })
    }

    #[test]
    fn parses_every_accepted_shape() {
        let table = [
            ("10,20", intent("", Some((10, 20)))),
            (" -9 , -9 ", intent("", Some((-9, -9)))),
            ("(3,4)", intent("", Some((3, 4)))),
            ("foo.dcl.eth", intent("foo.dcl.eth", None)),
            ("foo.dcl.eth 5,6", intent("foo.dcl.eth", Some((5, 6)))),
            ("foo.dcl.eth  -5, 6", intent("foo.dcl.eth", Some((-5, 6)))),
            (
                "http://192.168.1.20:8000",
                intent("http://192.168.1.20:8000", None),
            ),
            (
                "http://localhost:8000 2,2",
                intent("http://localhost:8000", Some((2, 2))),
            ),
            (
                "https://realm.example.com/main 1,1",
                intent("https://realm.example.com/main", Some((1, 1))),
            ),
            (
                "decentraland://open?position=-9,-9",
                intent("", Some((-9, -9))),
            ),
            (
                "https://decentraland.org/jump/?position=10%2C20",
                intent("", Some((10, 20))),
            ),
            (
                "https://decentraland.org/jump/?realm=foo.dcl.eth",
                intent("foo.dcl.eth", None),
            ),
            (
                "https://mobile.dclexplorer.com/open?realm=foo.dcl.eth&position=1,2",
                intent("foo.dcl.eth", Some((1, 2))),
            ),
            (
                "decentraland://open?preview=http://10.0.0.5:8000",
                intent("http://10.0.0.5:8000", None),
            ),
        ];
        for (text, expected) in table {
            assert_eq!(parse_input(text), expected, "{text:?}");
        }
    }

    #[test]
    fn rejects_what_names_nothing() {
        for text in [
            "",
            "   ",
            "two words",
            "main",
            "10,",
            ",10",
            "1,2,3",
            "99999999999,0",
            "foo 5,6",
            "decentraland://events?id=abc",
            "https://decentraland.org/events",
        ] {
            assert_eq!(parse_input(text), None, "{text:?}");
        }
    }
}
