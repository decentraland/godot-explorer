//! FFI for the avatar rules: the wearable catalog the edits resolve against and the fixed data the
//! editor shows. The edits themselves are `Control`s the core computes (`app`).

use super::{core, CoreError};
use crate::{
    app::ColorTarget,
    avatar::{self, BaseEmote, CatalogItem, OwnedEmote},
    profile::{self, Endpoints, Identity, ReqwestTransport, SessionIdentity},
};

/// Loads the base wearables once, adds them to the catalog the avatar edits use and returns them.
#[uniffi::export]
pub async fn load_avatar_catalog() -> Result<Vec<CatalogItem>, CoreError> {
    let core = core()?;
    let base = |app: &crate::app::App| -> Vec<CatalogItem> {
        let items = app.catalog().items().iter();
        items.filter(|item| !item.collectible).cloned().collect()
    };
    let loaded = core.with_app(|app| base(app));
    if !loaded.is_empty() {
        return Ok(loaded);
    }
    let items = core
        .handle()
        .spawn(
            async move { avatar::base_catalog(&ReqwestTransport::new(), &Endpoints::live()).await },
        )
        .await
        .map_err(invalid)?
        .map_err(invalid)?;
    core.with_app(|app| app.catalog_mut().merge(items.clone()));
    Ok(items)
}

/// What the account owns, on top of the base catalog.
#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct OwnedAvatarItems {
    pub wearables: Vec<CatalogItem>,
    pub emotes: Vec<OwnedEmote>,
}

/// Fetches the wearables and emotes the signed-in account owns and makes the wearables part of
/// the catalog; a guest owns none. Call it each time the editor opens: purchases show up.
#[uniffi::export]
pub async fn load_owned_avatar_items() -> Result<OwnedAvatarItems, CoreError> {
    let core = core()?;
    let identity = SessionIdentity;
    let address = identity.address().filter(|_| !identity.is_local_guest());
    let endpoints = profile::global().endpoints();
    let (wearables, emotes) = core
        .handle()
        .spawn(async move {
            let transport = ReqwestTransport::new();
            // Dev/testing links add fake-owned wearables, also for a guest.
            let fake = avatar::fake_owned_wearables(&transport, &endpoints).await?;
            let Some(address) = address else {
                return Ok((
                    (fake, avatar::TokenUrns::new()),
                    (Vec::new(), avatar::TokenUrns::new()),
                ));
            };
            let (mut wearables, tokens) =
                avatar::owned_wearables(&transport, &endpoints, &address).await?;
            wearables.extend(fake);
            let emotes = avatar::owned_emotes(&transport, &endpoints, &address).await?;
            Ok::<_, avatar::CatalogError>(((wearables, tokens), emotes))
        })
        .await
        .map_err(invalid)?
        .map_err(invalid)?;
    let ((wearables, mut tokens), (emotes, emote_tokens)) = (wearables, emotes);
    tokens.extend(emote_tokens);
    core.with_app(|app| app.catalog_mut().set_owned(wearables.clone(), tokens));
    Ok(OwnedAvatarItems { wearables, emotes })
}

/// Publishes the avatar the editor left on the player, and has the engine wear it in the world.
/// False when the profile already wore it.
#[uniffi::export]
pub async fn save_avatar() -> Result<bool, CoreError> {
    let core = core()?;
    let Some((avatar, tokens)) =
        core.with_app(|app| Some((app.avatar()?.clone(), app.avatar_tokens())))
    else {
        return Ok(false);
    };
    let service = profile::global();
    let saved = core
        .handle()
        .spawn(async move { service.deploy_avatar(&avatar, &tokens).await })
        .await
        .map_err(invalid)?
        .map_err(invalid)?;
    let Some(document) = saved else {
        return Ok(false);
    };
    let content = serde_json::Value::Object(document.content().clone()).to_string();
    core.with_app(|app| app.commit_avatar(content));
    Ok(true)
}

fn invalid(error: impl std::fmt::Display) -> CoreError {
    CoreError::InvalidCall {
        detail: error.to_string(),
    }
}

/// The colour swatches the editor offers.
#[uniffi::export]
pub fn avatar_palette(target: ColorTarget) -> Vec<String> {
    avatar::palette(target)
        .iter()
        .map(|c| c.to_string())
        .collect()
}

/// The emotes every account has.
#[uniffi::export]
pub fn base_emotes() -> Vec<BaseEmote> {
    avatar::base_emotes()
}

/// Whether the avatar may go without an item of this category.
#[uniffi::export]
pub fn is_removable_category(category: String) -> bool {
    avatar::is_removable(&category)
}
