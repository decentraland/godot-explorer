//! OpenSea assets, fetched through the Decentraland proxy and cached per NFT.

use std::cell::RefCell;
use std::collections::HashMap;

use godot::classes::http_client::Method;
use godot::classes::{RefCounted, Texture2D};
use godot::prelude::*;

use super::urn::NftUrn;
use crate::engine::local_task::{await_promise, spawn_local};
use crate::fetcher::globals;
use crate::godot_classes::dcl_urls::DclUrls;
use crate::godot_classes::promise::{Promise, PromiseError};

/// An NFT as OpenSea describes it. Resolved through a promise, so a `RefCounted`.
#[derive(GodotClass)]
#[class(no_init, base=RefCounted)]
pub struct NftAsset {
    pub details: AssetDetails,
    pub texture: Option<Gd<Texture2D>>,
    base: Base<RefCounted>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct AssetDetails {
    pub endpoint: String,
    pub name: String,
    pub description: String,
    pub opensea_url: String,
    pub contract_address: String,
    pub token_id: String,
    pub image_url: String,
    pub owner_address: String,
}

impl AssetDetails {
    /// The asset out of OpenSea's `{"nft": {...}}`; None when it lacks the contract, token id
    /// or image.
    pub fn parse(endpoint: &str, json: &serde_json::Value) -> Option<AssetDetails> {
        let nft = json.get("nft")?;
        let text = |key: &str| {
            nft.get(key)
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string()
        };
        let owner = nft
            .get("owners")
            .and_then(|o| o.as_array())
            .and_then(|o| o.first())
            .and_then(|o| o.get("address"))
            .and_then(|a| a.as_str())
            .unwrap_or_default()
            .to_string();
        let details = AssetDetails {
            endpoint: endpoint.to_string(),
            name: text("name"),
            description: text("description"),
            opensea_url: text("opensea_url"),
            contract_address: text("contract"),
            token_id: text("identifier"),
            image_url: text("image_url"),
            owner_address: owner,
        };
        let valid = !details.token_id.is_empty()
            && !details.image_url.is_empty()
            && !details.contract_address.is_empty();
        valid.then_some(details)
    }

    /// The texture cache key: an underscore, as a colon is invalid in file paths on macOS.
    pub fn hash(&self) -> String {
        format!("{}_{}", self.contract_address, self.token_id)
    }
}

thread_local! {
    static CACHE: RefCell<HashMap<String, Gd<Promise>>> = RefCell::new(HashMap::new());
}

/// The asset of an NFT, a promise resolved with an `NftAsset` or rejected.
pub fn fetch_nft(urn: &NftUrn) -> Gd<Promise> {
    let key = urn.hash();
    if let Some(cached) = CACHE.with(|cache| cache.borrow().get(&key).cloned()) {
        if cached.is_instance_valid() {
            return cached;
        }
    }
    let promise = Promise::new_alloc();
    CACHE.with(|cache| cache.borrow_mut().insert(key, promise.clone()));
    spawn_local(request_nft(promise.clone(), urn.clone()));
    promise
}

async fn request_nft(promise: Gd<Promise>, urn: NftUrn) {
    let url = format!(
        "{}/api/v2/chain/{}/contract/{}/nfts/{}",
        DclUrls::open_sea_proxy(),
        urn.chain,
        urn.contract_address,
        urn.token_id
    );
    let mut headers = VarDictionary::new();
    headers.set("Content-Type", "application/json");
    headers.set("X-API-KEY", "");
    let response = globals::http_requester().bind().request_json(
        url.as_str().into(),
        Method::GET,
        GString::new(),
        headers,
    );
    let result = await_promise(response).await;
    let mut promise = promise;
    if let Ok(error) = result.try_to::<Gd<PromiseError>>() {
        let message = format!(
            "Error fetching asset result for opensea, url: {url} error: {}",
            error.bind().get_error()
        );
        godot_error!("{message}");
        // Rejected, not left pending: the shape would wait on it forever.
        promise.bind_mut().reject(message.as_str().into());
        return;
    }
    let Ok(response) =
        result.try_to::<Gd<crate::http_request::request_response::RequestResponse>>()
    else {
        promise.bind_mut().reject("not a response".into());
        return;
    };
    let body = response.bind().get_response_as_string().to_string();
    let json: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();
    let Some(details) = AssetDetails::parse(&url, &json) else {
        let message = format!("Error on opensea nft asset: {}", urn.hash());
        godot_error!("{message}");
        promise.bind_mut().reject(message.as_str().into());
        return;
    };

    let texture = download_image(&details).await;
    let asset = Gd::from_init_fn(|base| NftAsset {
        details,
        texture,
        base,
    });
    promise.bind_mut().resolve_with_data(asset.to_variant());
}

async fn download_image(details: &AssetDetails) -> Option<Gd<Texture2D>> {
    let promise = globals::content_provider().bind_mut().fetch_texture_by_url(
        details.hash().as_str().into(),
        details.image_url.as_str().into(),
    );
    let result = await_promise(promise).await;
    if let Ok(error) = result.try_to::<Gd<PromiseError>>() {
        godot_error!(
            "open_sea_nft_fetcher::asset::download_image promise error: {}",
            error.bind().get_error()
        );
        return None;
    }
    let entry = result
        .try_to::<Gd<crate::content::texture::TextureEntry>>()
        .ok()?;
    let texture = entry.bind().texture.clone();
    Some(texture)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn json(text: &str) -> serde_json::Value {
        serde_json::from_str(text).unwrap()
    }

    #[test]
    fn parses_a_complete_asset() {
        let asset = AssetDetails::parse(
            "https://proxy/x",
            &json(
                r#"{"nft": {"contract": "0xabc", "identifier": "7", "name": "Ape",
                    "description": "d", "opensea_url": "https://opensea.io/x",
                    "image_url": "https://img/x.png",
                    "owners": [{"address": "0x1111111111111111111111111111111111111111"}]}}"#,
            ),
        )
        .unwrap();
        assert_eq!(asset.name, "Ape");
        assert_eq!(asset.hash(), "0xabc_7");
        assert_eq!(
            asset.owner_address,
            "0x1111111111111111111111111111111111111111"
        );
    }

    #[test]
    fn an_asset_without_an_image_or_ids_is_invalid() {
        assert!(AssetDetails::parse(
            "e",
            &json(r#"{"nft": {"contract": "0xabc", "identifier": "7"}}"#)
        )
        .is_none());
        assert!(AssetDetails::parse(
            "e",
            &json(r#"{"nft": {"image_url": "i", "identifier": "7"}}"#)
        )
        .is_none());
        assert!(AssetDetails::parse("e", &json(r#"{}"#)).is_none());
    }

    #[test]
    fn non_string_fields_read_as_empty() {
        let asset = AssetDetails::parse(
            "e",
            &json(r#"{"nft": {"contract": "0xabc", "identifier": "7", "image_url": "i", "name": null, "owners": []}}"#),
        )
        .unwrap();
        assert_eq!(asset.name, "");
        assert_eq!(asset.owner_address, "");
    }
}
