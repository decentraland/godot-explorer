use std::time::Duration;

use async_trait::async_trait;
use serde_json::Value;

use super::facts::SceneFacts;
use crate::{app::Parcel, realm, urls};

const REQUEST_TIMEOUT: Duration = Duration::from_secs(20);

/// What the shell shows for the scene the player is in.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct CurrentScene {
    pub title: String,
    pub creator: String,
    pub parcel: Parcel,
    pub realm: String,
    pub genesis: bool,
}

/// The fields of a destinations record the shell shows.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Place {
    pub title: String,
    pub creator: String,
}

/// HTTP seam for the destinations lookup, so tests run offline.
#[async_trait]
pub trait PlaceSource: Send + Sync {
    /// GETs `url` and returns the parsed JSON body of a 2xx response.
    async fn get_json(&self, url: &str) -> Result<Value, String>;
}

pub struct ReqwestPlaceSource {
    client: reqwest::Client,
}

impl ReqwestPlaceSource {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .user_agent(crate::USER_AGENT)
            .timeout(REQUEST_TIMEOUT)
            .build()
            .unwrap_or_default();
        Self { client }
    }
}

impl Default for ReqwestPlaceSource {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PlaceSource for ReqwestPlaceSource {
    async fn get_json(&self, url: &str) -> Result<Value, String> {
        let response = self
            .client
            .get(url)
            .header("Content-Type", "application/json")
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let status = response.status();
        if !status.is_success() {
            return Err(format!("status {status}"));
        }
        response.json().await.map_err(|e| e.to_string())
    }
}

pub fn place_by_position_url(parcel: Parcel) -> String {
    format!(
        "{}/?only_places=true&pointer={},{}",
        urls::destinations_api(),
        parcel.x,
        parcel.y
    )
}

pub fn place_by_names_url(name: &str) -> String {
    format!(
        "{}/?names={}&only_worlds=true&limit=1",
        urls::destinations_api(),
        realm::uri_encode(name)
    )
}

/// The first record of the response's `data` array; empty when there is none.
pub fn parse_place(body: &Value) -> Place {
    let Some(record) = body
        .get("data")
        .and_then(Value::as_array)
        .and_then(|data| data.first())
        .filter(|record| record.is_object())
    else {
        return Place::default();
    };
    let text = |key: &str| {
        record
            .get(key)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string()
    };
    Place {
        title: text("title"),
        creator: text("contact_name"),
    }
}

/// The scene the player is in. The destinations record's title wins over the scene's own when it
/// has one; a failed lookup leaves the scene's title and no creator.
pub async fn current_scene(facts: &SceneFacts, source: &dyn PlaceSource) -> CurrentScene {
    let genesis = realm::is_genesis_city(&facts.realm_url);
    let url = if genesis {
        place_by_position_url(facts.parcel)
    } else {
        place_by_names_url(&facts.realm_name)
    };
    let place = match source.get_json(&url).await {
        Ok(body) => parse_place(&body),
        Err(error) => {
            tracing::warn!("destinations lookup failed: {error}");
            Place::default()
        }
    };
    let title = if place.title.is_empty() {
        facts.scene_title.clone().unwrap_or_default()
    } else {
        place.title
    };
    CurrentScene {
        title,
        creator: place.creator,
        parcel: facts.parcel,
        realm: facts.realm_url.clone(),
        genesis,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use serde_json::json;

    use super::*;

    struct FakeSource {
        reply: Result<Value, String>,
        asked: Mutex<Vec<String>>,
    }

    impl FakeSource {
        fn replying(reply: Result<Value, String>) -> Self {
            Self {
                reply,
                asked: Mutex::new(Vec::new()),
            }
        }
    }

    #[async_trait]
    impl PlaceSource for FakeSource {
        async fn get_json(&self, url: &str) -> Result<Value, String> {
            self.asked.lock().unwrap().push(url.to_string());
            self.reply.clone()
        }
    }

    fn genesis_facts() -> SceneFacts {
        SceneFacts {
            realm_url: "https://peer-ec1.decentraland.org".into(),
            parcel: Parcel::new(-12, 40),
            scene_title: Some("Scene own title".into()),
            realm_name: String::new(),
        }
    }

    fn world_facts() -> SceneFacts {
        SceneFacts {
            realm_url: "https://worlds-content-server.decentraland.org/world/spacerunner.dcl.eth"
                .into(),
            parcel: Parcel::new(0, 0),
            scene_title: Some("World scene".into()),
            realm_name: "spacerunner.dcl.eth".into(),
        }
    }

    #[test]
    fn genesis_detection_table() {
        let cases = [
            ("https://peer-ec1.decentraland.org", true),
            ("https://peer-ec1.decentraland.org/", true),
            ("peer-ap1.decentraland.org", true),
            ("https://realm-provider-ea.decentraland.org/main", true),
            ("https://example.com", false),
            ("spacerunner.dcl.eth", false),
            ("http://192.168.1.20:8000", false),
            ("", false),
        ];
        for (realm_url, expected) in cases {
            assert_eq!(realm::is_genesis_city(realm_url), expected, "{realm_url}");
        }
    }

    #[test]
    fn lookup_urls() {
        assert!(place_by_position_url(Parcel::new(-12, 40))
            .ends_with("/destinations/?only_places=true&pointer=-12,40"));
        assert!(place_by_names_url("my world.dcl.eth")
            .ends_with("/destinations/?names=my%20world.dcl.eth&only_worlds=true&limit=1"));
    }

    #[test]
    fn parse_place_reads_the_first_record() {
        let body = json!({"data": [{"title": "Plaza", "contact_name": "Ana"}, {"title": "Other"}]});
        assert_eq!(
            parse_place(&body),
            Place {
                title: "Plaza".into(),
                creator: "Ana".into()
            }
        );
    }

    #[test]
    fn parse_place_tolerates_every_odd_shape() {
        for body in [
            json!({}),
            json!({"data": []}),
            json!({"data": {}}),
            json!({"data": [1]}),
            json!([]),
            json!({"data": [{"title": 5, "contact_name": null}]}),
        ] {
            assert_eq!(parse_place(&body), Place::default(), "{body}");
        }
    }

    #[tokio::test]
    async fn genesis_looks_up_by_position_and_the_place_title_wins() {
        let source = FakeSource::replying(Ok(
            json!({"data": [{"title": "Genesis Plaza", "contact_name": "Ana"}]}),
        ));
        let scene = current_scene(&genesis_facts(), &source).await;
        assert_eq!(
            scene,
            CurrentScene {
                title: "Genesis Plaza".into(),
                creator: "Ana".into(),
                parcel: Parcel::new(-12, 40),
                realm: "https://peer-ec1.decentraland.org".into(),
                genesis: true,
            }
        );
        let asked = source.asked.lock().unwrap();
        assert_eq!(asked.len(), 1);
        assert!(asked[0].contains("pointer=-12,40"));
    }

    #[tokio::test]
    async fn a_world_looks_up_by_name() {
        let source = FakeSource::replying(Ok(json!({"data": [{"title": "Space Runner"}]})));
        let scene = current_scene(&world_facts(), &source).await;
        assert!(!scene.genesis);
        assert_eq!(scene.title, "Space Runner");
        assert_eq!(scene.creator, "");
        assert!(source.asked.lock().unwrap()[0].contains("names=spacerunner.dcl.eth"));
    }

    #[tokio::test]
    async fn the_scene_title_stands_when_the_place_has_none() {
        let source =
            FakeSource::replying(Ok(json!({"data": [{"title": "", "contact_name": "Bo"}]})));
        let scene = current_scene(&genesis_facts(), &source).await;
        assert_eq!(scene.title, "Scene own title");
        assert_eq!(scene.creator, "Bo");
    }

    #[tokio::test]
    async fn a_failed_lookup_keeps_the_scene_title_and_no_creator() {
        let source = FakeSource::replying(Err("offline".into()));
        let scene = current_scene(&genesis_facts(), &source).await;
        assert_eq!(scene.title, "Scene own title");
        assert_eq!(scene.creator, "");
    }

    #[tokio::test]
    async fn no_scene_and_no_place_is_an_empty_title() {
        let source = FakeSource::replying(Ok(json!({"data": []})));
        let facts = SceneFacts {
            scene_title: None,
            ..genesis_facts()
        };
        assert_eq!(current_scene(&facts, &source).await.title, "");
    }
}
