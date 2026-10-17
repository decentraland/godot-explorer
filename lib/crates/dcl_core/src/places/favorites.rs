//! The signed part of the places service: the user's favorites, and favorite and vote writes.
//! Public lists stay in the shell (D-10).

use serde_json::{json, Value};

use super::signed::{build_url, SignedClient, SignedError};
use crate::urls;

/// The user's vote on a place.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Enum))]
pub enum Vote {
    Up,
    Down,
    None,
}

impl Vote {
    fn like(self) -> Value {
        match self {
            Self::Up => Value::Bool(true),
            Self::Down => Value::Bool(false),
            Self::None => Value::Null,
        }
    }
}

/// A place as the favorites list shows it. `base_position` is `x,y` for a parcel; for a world the
/// shell uses `world_name`.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct Place {
    pub id: String,
    pub title: String,
    /// The contact name, or the owner when there is none.
    pub creator: String,
    pub image: String,
    pub users: u32,
    pub like_rate: Option<f64>,
    pub world: bool,
    pub world_name: String,
    pub base_position: String,
    pub description: Option<String>,
    pub categories: Vec<String>,
}

/// What the signed-in user did with one place.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct PlaceUserState {
    pub favorite: bool,
    pub vote: Vote,
}

fn text(raw: &Value, key: &str) -> String {
    raw.get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

/// A place the list can show needs an id and a title.
fn parse_place(raw: &Value) -> Option<Place> {
    let id = text(raw, "id");
    let title = text(raw, "title");
    if id.is_empty() || title.is_empty() {
        return None;
    }
    let contact = text(raw, "contact_name");
    let description = text(raw, "description");
    Some(Place {
        id,
        title,
        creator: if contact.is_empty() {
            text(raw, "owner")
        } else {
            contact
        },
        image: text(raw, "image"),
        users: raw
            .get("user_count")
            .and_then(Value::as_u64)
            .unwrap_or_default() as u32,
        like_rate: raw.get("like_rate").and_then(Value::as_f64),
        world: raw.get("world").and_then(Value::as_bool) == Some(true),
        world_name: text(raw, "world_name"),
        base_position: text(raw, "base_position"),
        description: (!description.is_empty()).then_some(description),
        categories: raw
            .get("categories")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(|c| c.as_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or_default(),
    })
}

/// The places service answers `{ok, data}` with an array for lists and an object for one place.
fn rows(body: &Value) -> Vec<&Value> {
    match body.get("data") {
        Some(Value::Array(items)) => items.iter().collect(),
        Some(item @ Value::Object(_)) => vec![item],
        _ => Vec::new(),
    }
}

fn places_url(segments: &[&str], query: &[(&str, String)]) -> Result<String, SignedError> {
    build_url(&urls::places_api(), segments, query)
}

/// One page of the user's favorite places, optionally narrowed by a search text.
pub async fn favorite_places(
    client: &SignedClient,
    offset: u32,
    limit: u32,
    search: Option<&str>,
) -> Result<Vec<Place>, SignedError> {
    let mut query = vec![
        ("only_favorites", "true".to_owned()),
        ("limit", limit.to_string()),
    ];
    match search {
        Some(search) => query.push(("search", search.trim().to_owned())),
        None => query.push(("offset", offset.to_string())),
    }
    let body = client
        .request_json("GET", &places_url(&["places"], &query)?, None)
        .await?;
    Ok(rows(&body).into_iter().filter_map(parse_place).collect())
}

/// The user's favorite flag and vote on a place; `None` when the service does not know it.
pub async fn place_user_state(
    client: &SignedClient,
    place_id: &str,
) -> Result<Option<PlaceUserState>, SignedError> {
    let body = client
        .request_json("GET", &places_url(&["places", place_id], &[])?, None)
        .await?;
    Ok(rows(&body).first().map(|raw| {
        let flag = |key: &str| raw.get(key).and_then(Value::as_bool) == Some(true);
        let vote = if flag("user_like") {
            Vote::Up
        } else if flag("user_dislike") {
            Vote::Down
        } else {
            Vote::None
        };
        PlaceUserState {
            favorite: flag("user_favorite"),
            vote,
        }
    }))
}

fn kind(is_world: bool) -> &'static str {
    if is_world {
        "worlds"
    } else {
        "places"
    }
}

pub async fn set_favorite(
    client: &SignedClient,
    place_id: &str,
    value: bool,
    is_world: bool,
) -> Result<(), SignedError> {
    let url = places_url(&[kind(is_world), place_id, "favorites"], &[])?;
    client
        .request("PATCH", &url, Some(&json!({ "favorites": value })))
        .await
        .map(drop)
}

pub async fn vote(
    client: &SignedClient,
    place_id: &str,
    vote: Vote,
    is_world: bool,
) -> Result<(), SignedError> {
    let url = places_url(&[kind(is_world), place_id, "likes"], &[])?;
    client
        .request("PATCH", &url, Some(&json!({ "like": vote.like() })))
        .await
        .map(drop)
}

#[cfg(test)]
mod tests {
    use super::super::signed::testing::*;
    use super::*;

    #[tokio::test]
    async fn favorites_page_is_a_signed_get_with_typed_places() {
        let mock = Mock::ok(
            &json!({"ok": true, "data": [
                {"id": "p1", "title": "Plaza", "contact_name": "", "owner": "0xo", "user_count": 3,
                 "like_rate": 0.8, "base_position": "10,20", "categories": ["art", 1]},
                {"id": "", "title": "dropped"},
                {"id": "w1", "title": "World", "world": true, "world_name": "foo.dcl.eth",
                 "contact_name": "Bob", "description": "hi"}
            ]})
            .to_string(),
        );
        let places = favorite_places(&client(&mock), 40, 20, None).await.unwrap();
        let sent = mock.only_sent();
        assert_eq!(sent.method, "GET");
        assert!(sent.url.starts_with(&urls::places_api()));
        assert!(sent.url.contains("/places?"));
        assert!(sent.url.contains("only_favorites=true"));
        assert!(sent.url.contains("limit=20"));
        assert!(sent.url.contains("offset=40"));
        assert!(sent.header("x-identity-auth-chain-0").is_some());
        assert_eq!(places.len(), 2);
        assert_eq!(places[0].creator, "0xo");
        assert_eq!(places[0].users, 3);
        assert_eq!(places[0].like_rate, Some(0.8));
        assert_eq!(places[0].categories, vec!["art".to_owned()]);
        assert!(places[1].world);
        assert_eq!(places[1].creator, "Bob");
        assert_eq!(places[1].description.as_deref(), Some("hi"));
    }

    #[tokio::test]
    async fn favorites_search_sends_the_trimmed_text_and_no_offset() {
        let mock = Mock::ok(r#"{"ok":true,"data":[]}"#);
        favorite_places(&client(&mock), 0, 20, Some("  big plaza "))
            .await
            .unwrap();
        let url = mock.only_sent().url;
        assert!(url.contains("search=big+plaza"), "{url}");
        assert!(!url.contains("offset"), "{url}");
    }

    #[tokio::test]
    async fn user_state_reads_flags_of_the_single_place_object() {
        let mock = Mock::ok(
            r#"{"ok":true,"data":{"id":"p","title":"t","user_favorite":true,"user_dislike":true}}"#,
        );
        let state = place_user_state(&client(&mock), "a b").await.unwrap();
        assert!(mock.only_sent().url.ends_with("/places/a%20b"));
        assert_eq!(
            state,
            Some(PlaceUserState {
                favorite: true,
                vote: Vote::Down
            })
        );
    }

    #[tokio::test]
    async fn unknown_place_has_no_state() {
        let mock = Mock::ok(r#"{"ok":true,"data":[]}"#);
        assert_eq!(place_user_state(&client(&mock), "p").await.unwrap(), None);
    }

    #[tokio::test]
    async fn an_unparseable_answer_is_malformed() {
        let mock = Mock::ok("<html>");
        assert!(matches!(
            place_user_state(&client(&mock), "p").await,
            Err(SignedError::Malformed(_))
        ));
    }

    #[tokio::test]
    async fn favorite_patches_places_or_worlds_with_the_flag_signed() {
        for (is_world, segment) in [(false, "places"), (true, "worlds")] {
            let mock = Mock::ok("{}");
            set_favorite(&client(&mock), "id1", true, is_world)
                .await
                .unwrap();
            let sent = mock.only_sent();
            assert_eq!(sent.method, "PATCH");
            assert!(sent.url.ends_with(&format!("/{segment}/id1/favorites")));
            assert_eq!(sent.body.as_deref(), Some(r#"{"favorites":true}"#));
            assert_eq!(
                sent.header("x-identity-metadata"),
                Some(r#"{"favorites":true}"#)
            );
        }
    }

    #[tokio::test]
    async fn vote_maps_to_true_false_null() {
        for (vote_value, body) in [
            (Vote::Up, r#"{"like":true}"#),
            (Vote::Down, r#"{"like":false}"#),
            (Vote::None, r#"{"like":null}"#),
        ] {
            let mock = Mock::ok("{}");
            vote(&client(&mock), "id1", vote_value, false)
                .await
                .unwrap();
            let sent = mock.only_sent();
            assert!(sent.url.ends_with("/places/id1/likes"));
            assert_eq!(sent.body.as_deref(), Some(body));
        }
    }

    #[tokio::test]
    async fn a_rejected_write_surfaces_the_status() {
        let mock = Mock::answering(vec![Ok(super::super::signed::Reply {
            status: 403,
            body: "no".into(),
        })]);
        assert_eq!(
            set_favorite(&client(&mock), "p", false, false).await,
            Err(SignedError::Status {
                status: 403,
                body: "no".into()
            })
        );
    }
}
