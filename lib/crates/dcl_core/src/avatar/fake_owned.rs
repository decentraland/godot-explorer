//! Dev/testing: the wearables a `fake-owned-wearables` link adds to what the account owns, so the
//! Backpack can be tried with collectibles on any account, guests included. The same link turns
//! profile deploys off: a profile wearing items the wallet does not own would be rejected.

use std::sync::Mutex;

use http::Method;
use serde_json::{json, Value};

use super::catalog::{body_shapes_of, english_name, RARITIES};
use super::owned::{item_urn, thumbnail_of};
use super::service::CatalogError;
use super::CatalogItem;
use crate::profile::{Endpoints, HttpRequest, Transport};

static FAKE_OWNED: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// Replaces the fake-owned wearables (item or token URNs).
pub fn set_fake_owned(urns: Vec<String>) {
    *FAKE_OWNED.lock().unwrap_or_else(|e| e.into_inner()) = urns;
}

fn fake_owned() -> Vec<String> {
    FAKE_OWNED.lock().unwrap_or_else(|e| e.into_inner()).clone()
}

/// The fake-owned wearables as catalog items, from their entities on the content server.
pub async fn fake_owned_wearables(
    transport: &dyn Transport,
    endpoints: &Endpoints,
) -> Result<Vec<CatalogItem>, CatalogError> {
    let urns = fake_owned();
    if urns.is_empty() {
        return Ok(Vec::new());
    }
    let pointers: Vec<&str> = urns.iter().map(|urn| item_urn(urn)).collect();
    let reply = transport
        .send(HttpRequest {
            method: Method::POST,
            url: format!("{}entities/active", endpoints.content),
            headers: vec![("Content-Type".into(), "application/json".into())],
            body: json!({ "pointers": pointers }).to_string().into_bytes(),
        })
        .await
        .map_err(CatalogError::Network)?;
    if !(200..300).contains(&reply.status) {
        return Err(CatalogError::Http(reply.status));
    }
    Ok(parse_wearable_entities(&reply.body, &endpoints.content))
}

/// Wearable entities from `entities/active`; entities without category or thumbnail are dropped.
pub fn parse_wearable_entities(payload: &str, content_base: &str) -> Vec<CatalogItem> {
    let Ok(Value::Array(entities)) = serde_json::from_str::<Value>(payload) else {
        return Vec::new();
    };
    entities
        .iter()
        .filter_map(|entity| {
            let metadata = entity.get("metadata")?;
            let urn = metadata
                .get("id")
                .and_then(Value::as_str)
                .or_else(|| entity.get("pointers")?.as_array()?.first()?.as_str())?;
            let rarity = metadata
                .get("rarity")
                .and_then(Value::as_str)
                .filter(|rarity| RARITIES.contains(rarity))
                .unwrap_or("common");
            Some(CatalogItem {
                urn: urn.to_string(),
                name: english_name(metadata),
                category: metadata.pointer("/data/category")?.as_str()?.to_string(),
                thumbnail: thumbnail_of(entity, content_base)?,
                rarity: rarity.to_string(),
                body_shapes: body_shapes_of(metadata),
                collectible: true,
            })
        })
        .collect()
}
