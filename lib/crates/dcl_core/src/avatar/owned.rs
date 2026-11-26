//! The wearables and emotes an account owns, from the catalyst's `explorer` lambdas
//! (`/explorer/{address}/wearables` and `/emotes`, the routes Unity's explorer uses).

use std::collections::HashMap;

use http::Method;
use serde_json::Value;

use super::catalog::{body_shapes_of, english_name, RARITIES};
use super::service::CatalogError;
use super::CatalogItem;
use crate::profile::{Endpoints, HttpRequest, Transport};

const PAGE_SIZE: usize = 100;
/// Pages fetched at most; a server that keeps answering full pages must not loop forever.
const MAX_PAGES: usize = 50;

/// An emote the account owns.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct OwnedEmote {
    pub urn: String,
    pub name: String,
    pub thumbnail: String,
    pub rarity: String,
    pub category: String,
}

/// The token URN a profile must carry for an owned item, by item URN (lowercase). The catalyst
/// only accepts a collectible in a deployed profile with its token id (#2489).
pub type TokenUrns = HashMap<String, String>;

/// The item URN of a collectible's token URN (`urn:decentraland:<net>:collections-v2:<addr>:<item>:<token>`);
/// any other URN is returned as it is.
pub fn item_urn(urn: &str) -> &str {
    let mut parts = urn.match_indices(':');
    let is_collection = urn
        .split(':')
        .nth(3)
        .is_some_and(|kind| kind == "collections-v1" || kind == "collections-v2");
    match parts.nth(5) {
        Some((end, _)) if is_collection => &urn[..end],
        _ => urn,
    }
}

/// One page of the lambda's answer.
#[derive(Debug, PartialEq)]
pub struct OwnedPage<T> {
    pub items: Vec<T>,
    pub tokens: Vec<(String, String)>,
    pub total: usize,
    pub received: usize,
}

impl<T> Default for OwnedPage<T> {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            tokens: Vec::new(),
            total: 0,
            received: 0,
        }
    }
}

/// The content-server URL of the entity's thumbnail.
fn thumbnail_of(entity: &Value, content_base: &str) -> Option<String> {
    let file = entity.pointer("/metadata/thumbnail")?.as_str()?;
    let hash = entity
        .get("content")?
        .as_array()?
        .iter()
        .find(|entry| entry.get("file").and_then(Value::as_str) == Some(file))?
        .get("hash")?
        .as_str()?;
    Some(format!("{content_base}contents/{hash}"))
}

fn rarity_of(element: &Value) -> String {
    element
        .get("rarity")
        .and_then(Value::as_str)
        .filter(|rarity| RARITIES.contains(rarity))
        .unwrap_or("common")
        .to_string()
}

fn name_of(element: &Value, metadata: &Value) -> String {
    let name = english_name(metadata);
    if name.is_empty() {
        element
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string()
    } else {
        name
    }
}

/// The first token of each element: any one of them makes the item wearable.
fn first_token(element: &Value) -> Option<String> {
    element
        .get("individualData")?
        .as_array()?
        .iter()
        .find_map(|token| token.get("id").and_then(Value::as_str))
        .map(str::to_string)
}

fn parse_page<T>(
    payload: &str,
    mut item: impl FnMut(&str, &Value, &Value) -> Option<T>,
) -> OwnedPage<T> {
    let Ok(root) = serde_json::from_str::<Value>(payload) else {
        return OwnedPage::default();
    };
    let total = root
        .get("totalAmount")
        .and_then(Value::as_u64)
        .unwrap_or_default() as usize;
    let elements = root
        .get("elements")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut page = OwnedPage {
        total,
        received: elements.len(),
        ..OwnedPage::default()
    };
    for element in &elements {
        let Some(urn) = element.get("urn").and_then(Value::as_str) else {
            continue;
        };
        let Some(entity) = element.get("entity") else {
            continue;
        };
        let Some(parsed) = item(urn, element, entity) else {
            continue;
        };
        if let Some(token) = first_token(element).filter(|token| token != urn) {
            page.tokens.push((urn.to_lowercase(), token));
        }
        page.items.push(parsed);
    }
    page
}

/// A page of `/explorer/{address}/wearables`; elements without category or thumbnail are dropped.
pub fn parse_owned_wearables(payload: &str, content_base: &str) -> OwnedPage<CatalogItem> {
    parse_page(payload, |urn, element, entity| {
        let metadata = entity.get("metadata")?;
        let category = element
            .get("category")
            .and_then(Value::as_str)
            .or_else(|| metadata.pointer("/data/category").and_then(Value::as_str))?;
        Some(CatalogItem {
            urn: urn.to_string(),
            name: name_of(element, metadata),
            category: category.to_string(),
            thumbnail: thumbnail_of(entity, content_base)?,
            rarity: rarity_of(element),
            body_shapes: body_shapes_of(metadata),
            collectible: true,
        })
    })
}

/// A page of `/explorer/{address}/emotes`; elements without a thumbnail are dropped.
pub fn parse_owned_emotes(payload: &str, content_base: &str) -> OwnedPage<OwnedEmote> {
    parse_page(payload, |urn, element, entity| {
        let metadata = entity.get("metadata")?;
        let category = element
            .get("category")
            .and_then(Value::as_str)
            .or_else(|| {
                metadata
                    .pointer("/emoteDataADR74/category")
                    .and_then(Value::as_str)
            })
            .unwrap_or_default();
        Some(OwnedEmote {
            urn: urn.to_string(),
            name: name_of(element, metadata),
            thumbnail: thumbnail_of(entity, content_base)?,
            rarity: rarity_of(element),
            category: category.to_string(),
        })
    })
}

/// `{peer}/explorer/` for a catalyst whose lambdas live at `{peer}/lambdas/`.
pub fn explorer_base(lambdas: &str) -> String {
    let trimmed = lambdas.trim_end_matches('/');
    let peer = trimmed.strip_suffix("/lambdas").unwrap_or(trimmed);
    format!("{peer}/explorer/")
}

async fn fetch_all<T>(
    transport: &dyn Transport,
    url: impl Fn(usize) -> String,
    parse: impl Fn(&str) -> OwnedPage<T>,
) -> Result<(Vec<T>, TokenUrns), CatalogError> {
    let mut items = Vec::new();
    let mut tokens = TokenUrns::new();
    for page in 1..=MAX_PAGES {
        let reply = transport
            .send(HttpRequest {
                method: Method::GET,
                url: url(page),
                headers: Vec::new(),
                body: Vec::new(),
            })
            .await
            .map_err(CatalogError::Network)?;
        if !(200..300).contains(&reply.status) {
            return Err(CatalogError::Http(reply.status));
        }
        let parsed = parse(&reply.body);
        items.extend(parsed.items);
        tokens.extend(parsed.tokens);
        if parsed.received == 0 || page * PAGE_SIZE >= parsed.total {
            break;
        }
    }
    Ok((items, tokens))
}

/// Every wearable the account owns (on-chain and third-party; the base ones are the base catalog).
pub async fn owned_wearables(
    transport: &dyn Transport,
    endpoints: &Endpoints,
    address: &str,
) -> Result<(Vec<CatalogItem>, TokenUrns), CatalogError> {
    let base = explorer_base(&endpoints.lambdas);
    fetch_all(
        transport,
        |page| {
            format!(
                "{base}{address}/wearables?pageNum={page}&pageSize={PAGE_SIZE}\
                 &collectionType=on-chain&collectionType=third-party"
            )
        },
        |payload| parse_owned_wearables(payload, &endpoints.content),
    )
    .await
}

/// Every emote the account owns.
pub async fn owned_emotes(
    transport: &dyn Transport,
    endpoints: &Endpoints,
    address: &str,
) -> Result<(Vec<OwnedEmote>, TokenUrns), CatalogError> {
    let base = explorer_base(&endpoints.lambdas);
    fetch_all(
        transport,
        |page| format!("{base}{address}/emotes?pageNum={page}&pageSize={PAGE_SIZE}"),
        |payload| parse_owned_emotes(payload, &endpoints.content),
    )
    .await
}
