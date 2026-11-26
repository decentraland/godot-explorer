//! The wearables the editor can offer and what the rules need to know about each.

use serde_json::Value;

use super::owned::{item_urn, TokenUrns};
use super::taxonomy::is_base_wearable;

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct CatalogItem {
    pub urn: String,
    pub name: String,
    pub category: String,
    pub thumbnail: String,
    /// `base` for the free base avatars, else the collection rarity.
    pub rarity: String,
    /// Empty when the item fits every body shape.
    pub body_shapes: Vec<String>,
    pub collectible: bool,
}

impl CatalogItem {
    pub(super) fn fits(&self, body_shape: &str) -> bool {
        self.body_shapes.is_empty()
            || self
                .body_shapes
                .iter()
                .any(|shape| shape.eq_ignore_ascii_case(body_shape))
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Catalog {
    items: Vec<CatalogItem>,
    tokens: TokenUrns,
}

impl Catalog {
    pub fn new(items: Vec<CatalogItem>) -> Self {
        Self {
            items,
            tokens: TokenUrns::new(),
        }
    }

    pub fn items(&self) -> &[CatalogItem] {
        &self.items
    }

    /// The item by its URN or by one of its tokens' URNs.
    pub fn find(&self, urn: &str) -> Option<&CatalogItem> {
        let urn = item_urn(urn);
        self.items
            .iter()
            .find(|item| item.urn.eq_ignore_ascii_case(urn))
    }

    /// The owned token URN a deployed profile carries for each owned item.
    pub fn tokens(&self) -> &TokenUrns {
        &self.tokens
    }

    /// Replaces the owned items (an account's purchases change) and keeps the rest.
    pub fn set_owned(&mut self, owned: Vec<CatalogItem>, tokens: TokenUrns) {
        self.items.retain(|item| !item.collectible);
        self.merge(owned);
        self.tokens = tokens;
    }

    /// Adds the items whose urn is not there yet.
    pub fn merge(&mut self, more: Vec<CatalogItem>) {
        for item in more {
            if self.find(&item.urn).is_none() {
                self.items.push(item);
            }
        }
    }
}

pub(super) const RARITIES: [&str; 8] = [
    "common",
    "uncommon",
    "rare",
    "epic",
    "legendary",
    "exotic",
    "mythic",
    "unique",
];

pub(super) fn english_name(raw: &Value) -> String {
    raw.get("i18n")
        .and_then(Value::as_array)
        .and_then(|entries| {
            entries.iter().find_map(|entry| {
                (entry.get("code").and_then(Value::as_str) == Some("en"))
                    .then(|| entry.get("text").and_then(Value::as_str))
                    .flatten()
            })
        })
        .or_else(|| raw.get("name").and_then(Value::as_str))
        .unwrap_or_default()
        .to_string()
}

pub(super) fn body_shapes_of(raw: &Value) -> Vec<String> {
    let mut shapes: Vec<String> = Vec::new();
    let reps = raw
        .pointer("/data/representations")
        .and_then(Value::as_array)
        .into_iter()
        .flatten();
    for rep in reps {
        for shape in rep
            .get("bodyShapes")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
        {
            if !shapes.iter().any(|known| known == shape) {
                shapes.push(shape.to_string());
            }
        }
    }
    shapes
}

/// The lambda's `{ "wearables": [...] }` payload; entries without id, category or thumbnail are
/// dropped.
pub fn parse_wearables(payload: &str) -> Vec<CatalogItem> {
    let Ok(root) = serde_json::from_str::<Value>(payload) else {
        return Vec::new();
    };
    let Some(list) = root.get("wearables").and_then(Value::as_array) else {
        return Vec::new();
    };
    list.iter()
        .filter_map(|raw| {
            let urn = raw.get("id")?.as_str()?;
            let category = raw.pointer("/data/category")?.as_str()?;
            let thumbnail = raw.get("thumbnail")?.as_str()?;
            let rarity = raw
                .get("rarity")
                .and_then(Value::as_str)
                .filter(|rarity| RARITIES.contains(rarity))
                .unwrap_or("base");
            Some(CatalogItem {
                urn: urn.to_string(),
                name: english_name(raw),
                category: category.to_string(),
                thumbnail: thumbnail.to_string(),
                rarity: rarity.to_string(),
                body_shapes: body_shapes_of(raw),
                collectible: !is_base_wearable(urn),
            })
        })
        .collect()
}
