use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex, MutexGuard,
};

use http::Method;
use serde::Serialize;
use serde_json::{json, Value};

use super::{
    cid,
    identity::Identity,
    model::{Profile, ProfileDocument, ProfileUpdate},
    multipart,
    transport::{HttpReply, HttpRequest, Transport},
};
use crate::{session::wallet::sign_request, urls};

const NAMES_PAGE_SIZE: usize = 100;
/// Pages fetched at most; a server that keeps answering full pages must not loop forever.
const NAMES_MAX_PAGES: usize = 100;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProfileError {
    NotSignedIn,
    /// No profile is published for the address.
    NotFound,
    /// The request never got a reply.
    Network(String),
    /// The server answered with a failure; `message` is its own reason when it sent one.
    Http {
        status: u16,
        message: String,
    },
    InvalidResponse(String),
    Signing(String),
}

impl std::fmt::Display for ProfileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotSignedIn => write!(f, "not signed in"),
            Self::NotFound => write!(f, "profile not found"),
            Self::Network(e) => write!(f, "network error: {e}"),
            Self::Http { status, message } => write!(f, "HTTP {status}: {message}"),
            Self::InvalidResponse(e) => write!(f, "invalid response: {e}"),
            Self::Signing(e) => write!(f, "signing failed: {e}"),
        }
    }
}

impl std::error::Error for ProfileError {}

/// Where profiles are read and deployed; both end with a slash.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Endpoints {
    pub lambdas: String,
    pub content: String,
}

impl Endpoints {
    pub fn new(lambdas: &str, content: &str) -> Self {
        Self {
            lambdas: with_slash(lambdas),
            content: with_slash(content),
        }
    }

    /// The configured catalyst, or the profile-group override when one is set.
    pub fn live() -> Self {
        Self::new(
            &urls::identity_lambdas().unwrap_or_else(urls::peer_lambdas),
            &urls::identity_content().unwrap_or_else(urls::peer_content),
        )
    }
}

fn with_slash(url: &str) -> String {
    format!("{}/", url.trim_end_matches('/'))
}

// ADR-290: no snapshot content files; the profile-images service renders the picture.
#[derive(Serialize)]
struct Deployment {
    version: &'static str,
    #[serde(rename = "type")]
    ty: &'static str,
    pointers: Vec<String>,
    timestamp: u128,
    content: Vec<()>,
    metadata: Value,
}

struct Own {
    address: String,
    document: ProfileDocument,
}

#[derive(Default)]
struct NamesCache {
    address: String,
    names: Vec<String>,
}

pub struct ProfileService {
    transport: Arc<dyn Transport>,
    identity: Arc<dyn Identity>,
    endpoints: Mutex<Endpoints>,
    own: Mutex<Option<Own>>,
    names: Mutex<Option<NamesCache>>,
    deploy_disabled: AtomicBool,
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|e| e.into_inner())
}

impl ProfileService {
    pub fn new(
        transport: Arc<dyn Transport>,
        identity: Arc<dyn Identity>,
        endpoints: Endpoints,
    ) -> Self {
        Self {
            transport,
            identity,
            endpoints: Mutex::new(endpoints),
            own: Mutex::new(None),
            names: Mutex::new(None),
            deploy_disabled: AtomicBool::new(false),
        }
    }

    /// Follows a realm change: its catalyst serves the profiles from then on.
    pub fn set_endpoints(&self, endpoints: Endpoints) {
        *lock(&self.endpoints) = endpoints;
    }

    /// Dev switch: deploys update the local profile only and never reach the catalyst.
    pub fn set_deploy_disabled(&self, disabled: bool) {
        self.deploy_disabled.store(disabled, Ordering::Relaxed);
    }

    /// Forgets the signed-in account's cached profile and names (sign-out, account change).
    pub fn reset(&self) {
        *lock(&self.own) = None;
        *lock(&self.names) = None;
    }

    /// Hands over the profile the engine already holds, so the next deploy builds on it.
    pub fn adopt(&self, document: ProfileDocument) -> Result<(), ProfileError> {
        let address = self.identity.address().ok_or(ProfileError::NotSignedIn)?;
        *lock(&self.own) = Some(Own { address, document });
        Ok(())
    }

    /// A profile: the signed-in account's own when `address` is `None` or its own, any other
    /// address is looked up on the catalyst.
    pub async fn profile(&self, address: Option<&str>) -> Result<Profile, ProfileError> {
        let own = self.identity.address();
        match address.filter(|a| !a.is_empty()) {
            Some(other) if own.as_deref().map(str::to_lowercase) != Some(other.to_lowercase()) => {
                Ok(self.fetch_document(other).await?.profile())
            }
            _ => Ok(self.own_document().await?.profile()),
        }
    }

    async fn own_document(&self) -> Result<ProfileDocument, ProfileError> {
        let address = self.identity.address().ok_or(ProfileError::NotSignedIn)?;
        if let Some(own) = lock(&self.own).as_ref().filter(|o| o.address == address) {
            return Ok(own.document.clone());
        }
        let document = if self.identity.is_local_guest() {
            ProfileDocument::default_for(&address)
        } else {
            match self.fetch_document(&address).await {
                Ok(document) => document,
                Err(ProfileError::NotFound) => ProfileDocument::default_for(&address),
                Err(e) => return Err(e),
            }
        };
        *lock(&self.own) = Some(Own {
            address,
            document: document.clone(),
        });
        Ok(document)
    }

    async fn fetch_document(&self, address: &str) -> Result<ProfileDocument, ProfileError> {
        let url = format!("{}profiles/{}", lock(&self.endpoints).lambdas, address);
        let reply = self.get(&url).await?;
        let mut body: Value = parse(&reply)?;
        let first = body["avatars"]
            .as_array_mut()
            .and_then(|avatars| (!avatars.is_empty()).then(|| avatars.remove(0)));
        match first {
            Some(Value::Object(content)) => Ok(ProfileDocument::from_content(content)),
            Some(_) => Err(ProfileError::InvalidResponse(
                "profile is not an object".into(),
            )),
            None => Err(ProfileError::NotFound),
        }
    }

    /// The claimed names the account owns.
    pub async fn owned_names(&self) -> Result<Vec<String>, ProfileError> {
        let address = self.identity.address().ok_or(ProfileError::NotSignedIn)?;
        if let Some(cache) = lock(&self.names).as_ref().filter(|c| c.address == address) {
            return Ok(cache.names.clone());
        }
        let base = lock(&self.endpoints).lambdas.clone();
        let mut names = Vec::new();
        for page in 1..=NAMES_MAX_PAGES {
            let url =
                format!("{base}users/{address}/names?pageNum={page}&pageSize={NAMES_PAGE_SIZE}");
            let body: Value = parse(&self.get(&url).await?)?;
            let elements = body["elements"].as_array().cloned().unwrap_or_default();
            let total = body["totalAmount"].as_u64().unwrap_or(0) as usize;
            let received = elements.len();
            names.extend(
                elements
                    .iter()
                    .filter_map(|e| e["name"].as_str().map(str::to_owned)),
            );
            if received == 0 || page * NAMES_PAGE_SIZE >= total {
                break;
            }
        }
        *lock(&self.names) = Some(NamesCache {
            address,
            names: names.clone(),
        });
        Ok(names)
    }

    /// Applies `update` to the own profile and publishes it with the version bumped. The cached
    /// profile only changes once the catalyst accepted the deployment. A local guest's profile
    /// is updated on the device alone.
    pub async fn deploy(&self, update: &ProfileUpdate) -> Result<Profile, ProfileError> {
        let address = self.identity.address().ok_or(ProfileError::NotSignedIn)?;
        let current = self.own_document().await?;
        let next = current
            .updated(update)
            .with_address(&address)
            .with_version(current.version() + 1);
        if !self.identity.is_local_guest() && !self.deploy_disabled.load(Ordering::Relaxed) {
            self.publish(&address, &next).await?;
        }
        *lock(&self.own) = Some(Own {
            address,
            document: next.clone(),
        });
        Ok(next.profile())
    }

    async fn publish(&self, address: &str, document: &ProfileDocument) -> Result<(), ProfileError> {
        let chain = self
            .identity
            .auth_chain()
            .ok_or(ProfileError::NotSignedIn)?;
        let entity = serde_json::to_string(&Deployment {
            version: "v3",
            ty: "profile",
            pointers: vec![address.to_owned()],
            timestamp: crate::clock::unix_time_ms(),
            content: vec![],
            metadata: json!({ "avatars": [document.deployable()] }),
        })
        .map_err(|e| ProfileError::InvalidResponse(e.to_string()))?;
        let id = cid::entity_id(entity.as_bytes());

        let signature = chain
            .ephemeral_wallet()
            .sign_message(id.clone())
            .await
            .map_err(|e| ProfileError::Signing(e.to_string()))?;
        let mut auth_chain = chain.auth_chain().clone();
        auth_chain.add_signed_entity(id.clone(), signature);

        let mut fields = vec![("entityId".to_owned(), id.clone())];
        fields.extend(auth_chain.formdata());
        let form = multipart::build(&fields, &id, entity.as_bytes());

        let url = format!("{}entities/", lock(&self.endpoints).content);
        let reply = self
            .send(HttpRequest {
                method: Method::POST,
                url,
                headers: vec![("Content-Type".into(), form.content_type)],
                body: form.body,
            })
            .await?;
        let accepted: Value = parse(&reply)?;
        if accepted.get("creationTimestamp").is_none() {
            return Err(ProfileError::InvalidResponse(
                "missing creationTimestamp".into(),
            ));
        }
        Ok(())
    }

    /// Asks the server to delete the account (a request it processes, not an immediate wipe).
    pub async fn request_account_deletion(&self) -> Result<(), ProfileError> {
        let chain = self
            .identity
            .auth_chain()
            .ok_or(ProfileError::NotSignedIn)?;
        let url = urls::account_deletion();
        let uri: http::Uri = url
            .parse()
            .map_err(|e: http::uri::InvalidUri| ProfileError::InvalidResponse(e.to_string()))?;
        let headers = sign_request("POST", &uri, &chain, json!({})).await;
        let reply = self
            .send(HttpRequest {
                method: Method::POST,
                url,
                headers,
                body: Vec::new(),
            })
            .await?;
        let body: Value = parse(&reply)?;
        if body["ok"].as_bool() == Some(true) {
            Ok(())
        } else {
            Err(ProfileError::Http {
                status: reply.status,
                message: body["error"].as_str().unwrap_or("invalid response").into(),
            })
        }
    }

    async fn get(&self, url: &str) -> Result<HttpReply, ProfileError> {
        self.send(HttpRequest {
            method: Method::GET,
            url: url.to_owned(),
            headers: Vec::new(),
            body: Vec::new(),
        })
        .await
    }

    /// Sends a request; a failure status is an error carrying the server's reason.
    async fn send(&self, request: HttpRequest) -> Result<HttpReply, ProfileError> {
        let reply = self
            .transport
            .send(request)
            .await
            .map_err(ProfileError::Network)?;
        if (200..300).contains(&reply.status) {
            return Ok(reply);
        }
        if reply.status == 404 {
            return Err(ProfileError::NotFound);
        }
        let message = serde_json::from_str::<Value>(&reply.body)
            .ok()
            .and_then(|v| v["error"].as_str().map(str::to_owned))
            .unwrap_or_else(|| reply.body.clone());
        Err(ProfileError::Http {
            status: reply.status,
            message,
        })
    }
}

fn parse(reply: &HttpReply) -> Result<Value, ProfileError> {
    serde_json::from_str(&reply.body).map_err(|e| ProfileError::InvalidResponse(e.to_string()))
}
