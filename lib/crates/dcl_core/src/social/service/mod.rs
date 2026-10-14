//! The friends rules: a service created lazily from the session, paged lists, the nearby list and
//! the `friends_changed` notification. The calls to the social service are behind `SocialApi`.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex, OnceLock},
    time::Duration,
};

use futures_util::future::join_all;
use tokio::runtime::Handle;
use tokio_util::sync::CancellationToken;

use super::{
    api::{ManagerApi, SocialApi},
    profile_lookup::{LambdaProfileSource, ProfileSource},
    social_service_manager::SocialServiceManager,
    types::{Friend, FriendRequests, NearbyUser, SocialError},
};
use crate::Core;

pub const PAGE_SIZE: u32 = 100;
pub const NEARBY_LIMIT: usize = 30;

const RESUBSCRIBE_DELAY: Duration = Duration::from_secs(2);
const RESUBSCRIBE_MAX_DELAY: Duration = Duration::from_secs(60);

/// Nearby players are the engine's data: it knows who is rendered around the player.
pub trait NearbyProvider: Send + Sync {
    fn nearby_addresses(&self) -> Vec<String>;
    /// The engine's local blocked list.
    fn is_blocked(&self, address: &str) -> bool;
}

/// Called from a core thread whenever the friend or request lists changed on the server, and
/// once after the update stream was re-established (updates may have been missed).
pub type FriendsChangedListener = Arc<dyn Fn() + Send + Sync>;

type ApiFactory = Box<dyn Fn() -> Option<Arc<dyn SocialApi>> + Send + Sync>;

#[derive(Default)]
struct Live {
    api: Option<Arc<dyn SocialApi>>,
    subscription: Option<CancellationToken>,
}

#[derive(Default)]
struct Listeners {
    next_id: u64,
    by_id: HashMap<u64, FriendsChangedListener>,
}

pub struct Friends {
    factory: ApiFactory,
    profiles: Arc<dyn ProfileSource>,
    handle: Handle,
    resubscribe_delay: Duration,
    live: Mutex<Live>,
    listeners: Arc<Mutex<Listeners>>,
    nearby: Mutex<Option<Arc<dyn NearbyProvider>>>,
}

static GLOBAL: OnceLock<Friends> = OnceLock::new();

/// The process-wide service, backed by the signed-in session. Needs an installed core.
pub fn friends() -> &'static Friends {
    GLOBAL.get_or_init(|| {
        let core = Core::get();
        Friends::new(
            Box::new(|| {
                let chain = Core::get().with_session(|s| s.ephemeral_auth_chain().cloned())?;
                Some(Arc::new(ManagerApi(SocialServiceManager::new(Arc::new(
                    chain,
                )))))
            }),
            Arc::new(LambdaProfileSource::new()),
            core.handle().clone(),
        )
    })
}

impl Friends {
    pub fn new(factory: ApiFactory, profiles: Arc<dyn ProfileSource>, handle: Handle) -> Self {
        Self {
            factory,
            profiles,
            handle,
            resubscribe_delay: RESUBSCRIBE_DELAY,
            live: Mutex::new(Live::default()),
            listeners: Arc::new(Mutex::new(Listeners::default())),
            nearby: Mutex::new(None),
        }
    }

    #[cfg(test)]
    fn with_resubscribe_delay(mut self, delay: Duration) -> Self {
        self.resubscribe_delay = delay;
        self
    }

    /// Creates the service from the session and starts the update stream. A no-op when it exists.
    pub fn start(&self) -> Result<(), SocialError> {
        self.api().map(|_| ())
    }

    /// Drops the service and its update stream. Called when the session ends.
    pub fn reset(&self) {
        let mut live = self.live.lock().unwrap();
        if let Some(token) = live.subscription.take() {
            token.cancel();
        }
        live.api = None;
    }

    pub fn add_friends_changed_listener(&self, listener: FriendsChangedListener) -> u64 {
        let mut listeners = self.listeners.lock().unwrap();
        listeners.next_id += 1;
        let id = listeners.next_id;
        listeners.by_id.insert(id, listener);
        id
    }

    pub fn remove_friends_changed_listener(&self, id: u64) {
        self.listeners.lock().unwrap().by_id.remove(&id);
    }

    pub fn set_nearby_provider(&self, provider: Arc<dyn NearbyProvider>) {
        *self.nearby.lock().unwrap() = Some(provider);
    }

    fn api(&self) -> Result<Arc<dyn SocialApi>, SocialError> {
        let mut live = self.live.lock().unwrap();
        if let Some(api) = &live.api {
            return Ok(api.clone());
        }
        let api = (self.factory)().ok_or(SocialError::NotSignedIn)?;
        let token = CancellationToken::new();
        self.handle.spawn(keep_subscribed(
            api.clone(),
            token.clone(),
            self.listeners.clone(),
            self.resubscribe_delay,
        ));
        live.api = Some(api.clone());
        live.subscription = Some(token);
        Ok(api)
    }

    pub async fn friends(&self, limit: u32, offset: u32) -> Result<Vec<Friend>, SocialError> {
        self.api()?.friends(limit, offset).await
    }

    pub async fn friend_requests(&self) -> Result<FriendRequests, SocialError> {
        let api = self.api()?;
        let received = api.received_requests(PAGE_SIZE, 0).await?;
        let sent = api.sent_requests(PAGE_SIZE, 0).await?;
        Ok(FriendRequests { received, sent })
    }

    pub async fn send_request(&self, address: &str, message: &str) -> Result<(), SocialError> {
        let message = (!message.is_empty()).then(|| message.to_string());
        self.api()?.send_request(address, message).await
    }

    pub async fn accept(&self, address: &str) -> Result<(), SocialError> {
        self.api()?.accept(address).await
    }

    pub async fn reject(&self, address: &str) -> Result<(), SocialError> {
        self.api()?.reject(address).await
    }

    pub async fn cancel(&self, address: &str) -> Result<(), SocialError> {
        self.api()?.cancel(address).await
    }

    pub async fn block(&self, address: &str) -> Result<(), SocialError> {
        self.api()?.block(address).await
    }

    pub async fn unblock(&self, address: &str) -> Result<(), SocialError> {
        self.api()?.unblock(address).await
    }

    /// Up to `NEARBY_LIMIT` players around, blocked ones left out, with their public profile. A
    /// profile that cannot be fetched shows the address as the name.
    pub async fn nearby(&self) -> Vec<NearbyUser> {
        let Some(provider) = self.nearby.lock().unwrap().clone() else {
            return Vec::new();
        };
        let addresses: Vec<String> = provider
            .nearby_addresses()
            .into_iter()
            .filter(|a| !a.is_empty() && !provider.is_blocked(a))
            .take(NEARBY_LIMIT)
            .collect();
        join_all(addresses.into_iter().map(|address| async move {
            let profile = self.profiles.public_profile(&address).await;
            match profile {
                Some(p) => NearbyUser {
                    address,
                    name: p.name,
                    has_claimed_name: p.has_claimed_name,
                    profile_picture_url: p.face_url,
                },
                None => NearbyUser {
                    name: address.clone(),
                    address,
                    ..NearbyUser::default()
                },
            }
        }))
        .await
    }
}

fn notify(listeners: &Mutex<Listeners>) {
    let all: Vec<FriendsChangedListener> =
        listeners.lock().unwrap().by_id.values().cloned().collect();
    for listener in all {
        listener();
    }
}

/// Follows the update stream, re-subscribing with a growing delay when it drops, until cancelled.
async fn keep_subscribed(
    api: Arc<dyn SocialApi>,
    token: CancellationToken,
    listeners: Arc<Mutex<Listeners>>,
    base_delay: Duration,
) {
    let mut delay = base_delay;
    let mut resubscribed = false;
    loop {
        let subscribed = tokio::select! {
            _ = token.cancelled() => return,
            result = api.subscribe() => result,
        };
        match subscribed {
            Ok(mut updates) => {
                delay = base_delay;
                if resubscribed {
                    notify(&listeners);
                }
                resubscribed = true;
                loop {
                    tokio::select! {
                        _ = token.cancelled() => return,
                        update = updates.recv() => match update {
                            Some(()) => notify(&listeners),
                            None => break,
                        },
                    }
                }
                tracing::debug!("friendship updates dropped, resubscribing");
            }
            Err(e) => tracing::debug!("friendship subscription failed: {e}"),
        }
        tokio::select! {
            _ = token.cancelled() => return,
            _ = tokio::time::sleep(delay) => {}
        }
        delay = (delay * 2).min(RESUBSCRIBE_MAX_DELAY);
    }
}

#[cfg(test)]
mod tests;
