//! Fetching the base wearables from the catalyst.

use http::Method;

use super::{parse_wearables, CatalogItem};
use crate::profile::{Endpoints, HttpRequest, Transport};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CatalogError {
    Network(String),
    Http(u16),
    /// The answer held no usable wearable.
    Empty,
}

impl std::fmt::Display for CatalogError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Network(e) => write!(f, "network error: {e}"),
            Self::Http(status) => write!(f, "wearables catalog responded {status}"),
            Self::Empty => write!(f, "wearables catalog is empty"),
        }
    }
}

impl std::error::Error for CatalogError {}

/// The free base wearables every account can put on.
pub async fn base_catalog(
    transport: &dyn Transport,
    endpoints: &Endpoints,
) -> Result<Vec<CatalogItem>, CatalogError> {
    let reply = transport
        .send(HttpRequest {
            method: Method::GET,
            url: format!(
                "{}collections/wearables?collectionId=urn:decentraland:off-chain:base-avatars",
                endpoints.lambdas
            ),
            headers: Vec::new(),
            body: Vec::new(),
        })
        .await
        .map_err(CatalogError::Network)?;
    if !(200..300).contains(&reply.status) {
        return Err(CatalogError::Http(reply.status));
    }
    let items = parse_wearables(&reply.body);
    if items.is_empty() {
        return Err(CatalogError::Empty);
    }
    Ok(items)
}
