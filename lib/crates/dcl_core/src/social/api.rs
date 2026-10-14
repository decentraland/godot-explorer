//! The calls the friends service makes on the social service, behind a trait so the rules above
//! it run against a fake. `ManagerApi` is the real one over `SocialServiceManager`.

use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::mpsc::UnboundedReceiver;

use super::{
    social_service_manager::SocialServiceManager,
    types::{Friend, SocialError},
};
use crate::proto::social_service::v2::{
    block_user_response, friendship_update, paginated_friendship_requests_response,
    unblock_user_response, upsert_friendship_response, FriendProfile, Pagination,
};

const SUBSCRIBE_TIMEOUT: Duration = Duration::from_secs(15);

/// A friend coming online or going offline.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Connectivity {
    pub address: String,
    pub online: bool,
}

#[async_trait]
pub trait SocialApi: Send + Sync {
    /// `limit == 0` asks for the service's default page.
    async fn friends(&self, limit: u32, offset: u32) -> Result<Vec<Friend>, SocialError>;
    async fn received_requests(&self, limit: u32, offset: u32) -> Result<Vec<Friend>, SocialError>;
    async fn sent_requests(&self, limit: u32, offset: u32) -> Result<Vec<Friend>, SocialError>;
    async fn send_request(&self, address: &str, message: Option<String>)
        -> Result<(), SocialError>;
    async fn accept(&self, address: &str) -> Result<(), SocialError>;
    async fn reject(&self, address: &str) -> Result<(), SocialError>;
    async fn cancel(&self, address: &str) -> Result<(), SocialError>;
    async fn block(&self, address: &str) -> Result<(), SocialError>;
    async fn unblock(&self, address: &str) -> Result<(), SocialError>;
    async fn blocked_users(&self, limit: u32, offset: u32) -> Result<Vec<Friend>, SocialError>;
    /// A friend coming online or going offline; the channel closes when the stream drops.
    async fn subscribe_connectivity(&self) -> Result<UnboundedReceiver<Connectivity>, SocialError>;
    /// One `()` per friendship update that changes a list; the channel closes when the stream
    /// drops.
    async fn subscribe(&self) -> Result<UnboundedReceiver<()>, SocialError>;
}

fn pagination(limit: u32, offset: u32) -> Option<Pagination> {
    (limit > 0).then_some(Pagination {
        limit: limit as i32,
        offset: offset as i32,
    })
}

fn friend(profile: FriendProfile, friendship_id: String) -> Friend {
    Friend {
        address: profile.address,
        name: profile.name,
        has_claimed_name: profile.has_claimed_name,
        profile_picture_url: profile.profile_picture_url,
        friendship_id,
        online: false,
    }
}

fn requests(
    response: crate::proto::social_service::v2::PaginatedFriendshipRequestsResponse,
) -> Result<Vec<Friend>, SocialError> {
    match response.response {
        Some(paginated_friendship_requests_response::Response::Requests(list)) => Ok(list
            .requests
            .into_iter()
            .filter_map(|r| Some(friend(r.friend?, r.id)))
            .collect()),
        Some(paginated_friendship_requests_response::Response::InternalServerError(e)) => Err(
            SocialError::request(e.message.unwrap_or_else(|| "internal server error".into())),
        ),
        None => Ok(Vec::new()),
    }
}

fn rpc(what: &str, e: anyhow::Error) -> SocialError {
    SocialError::request(format!("{what}: {e}"))
}

fn upsert(
    response: crate::proto::social_service::v2::UpsertFriendshipResponse,
) -> Result<(), SocialError> {
    use upsert_friendship_response::Response;
    match response.response {
        Some(Response::Accepted(_)) => Ok(()),
        Some(Response::InvalidFriendshipAction(e)) => Err(SocialError::request(
            e.message
                .unwrap_or_else(|| "invalid friendship action".into()),
        )),
        Some(Response::InternalServerError(e)) => Err(SocialError::request(
            e.message.unwrap_or_else(|| "internal server error".into()),
        )),
        Some(Response::InvalidRequest(e)) => Err(SocialError::request(
            e.message.unwrap_or_else(|| "invalid request".into()),
        )),
        None => Err(SocialError::request("empty response")),
    }
}

pub struct ManagerApi(pub SocialServiceManager);

#[async_trait]
impl SocialApi for ManagerApi {
    async fn friends(&self, limit: u32, offset: u32) -> Result<Vec<Friend>, SocialError> {
        let response = self
            .0
            .get_friends(pagination(limit, offset), None)
            .await
            .map_err(|e| rpc("get friends", e))?;
        Ok(response
            .friends
            .into_iter()
            .map(|p| friend(p, String::new()))
            .collect())
    }

    async fn received_requests(&self, limit: u32, offset: u32) -> Result<Vec<Friend>, SocialError> {
        let response = self
            .0
            .get_pending_friendship_requests(pagination(limit, offset))
            .await
            .map_err(|e| rpc("get pending requests", e))?;
        requests(response)
    }

    async fn sent_requests(&self, limit: u32, offset: u32) -> Result<Vec<Friend>, SocialError> {
        let response = self
            .0
            .get_sent_friendship_requests(pagination(limit, offset))
            .await
            .map_err(|e| rpc("get sent requests", e))?;
        requests(response)
    }

    async fn send_request(
        &self,
        address: &str,
        message: Option<String>,
    ) -> Result<(), SocialError> {
        let response = self
            .0
            .send_friend_request(address.to_string(), message)
            .await
            .map_err(|e| rpc("send friend request", e))?;
        upsert(response)
    }

    async fn accept(&self, address: &str) -> Result<(), SocialError> {
        let response = self
            .0
            .accept_friend_request(address.to_string())
            .await
            .map_err(|e| rpc("accept friend request", e))?;
        upsert(response)
    }

    async fn reject(&self, address: &str) -> Result<(), SocialError> {
        let response = self
            .0
            .reject_friend_request(address.to_string())
            .await
            .map_err(|e| rpc("reject friend request", e))?;
        upsert(response)
    }

    async fn cancel(&self, address: &str) -> Result<(), SocialError> {
        let response = self
            .0
            .cancel_friend_request(address.to_string())
            .await
            .map_err(|e| rpc("cancel friend request", e))?;
        upsert(response)
    }

    async fn block(&self, address: &str) -> Result<(), SocialError> {
        use block_user_response::Response;
        let response = self
            .0
            .block_user(address.to_string())
            .await
            .map_err(|e| rpc("block user", e))?;
        match response.response {
            Some(Response::Ok(_)) => Ok(()),
            Some(Response::InternalServerError(e)) => Err(SocialError::request(
                e.message.unwrap_or_else(|| "internal server error".into()),
            )),
            Some(Response::InvalidRequest(e)) => Err(SocialError::request(
                e.message.unwrap_or_else(|| "invalid request".into()),
            )),
            Some(Response::ProfileNotFound(e)) => Err(SocialError::request(
                e.message.unwrap_or_else(|| "profile not found".into()),
            )),
            None => Err(SocialError::request("empty response")),
        }
    }

    async fn unblock(&self, address: &str) -> Result<(), SocialError> {
        use unblock_user_response::Response;
        let response = self
            .0
            .unblock_user(address.to_string())
            .await
            .map_err(|e| rpc("unblock user", e))?;
        match response.response {
            Some(Response::Ok(_)) => Ok(()),
            Some(Response::InternalServerError(e)) => Err(SocialError::request(
                e.message.unwrap_or_else(|| "internal server error".into()),
            )),
            Some(Response::InvalidRequest(e)) => Err(SocialError::request(
                e.message.unwrap_or_else(|| "invalid request".into()),
            )),
            Some(Response::ProfileNotFound(e)) => Err(SocialError::request(
                e.message.unwrap_or_else(|| "profile not found".into()),
            )),
            None => Err(SocialError::request("empty response")),
        }
    }

    async fn blocked_users(&self, limit: u32, offset: u32) -> Result<Vec<Friend>, SocialError> {
        let response = self
            .0
            .get_blocked_users(pagination(limit, offset))
            .await
            .map_err(|e| rpc("get blocked users", e))?;
        Ok(response
            .profiles
            .into_iter()
            .map(|p| Friend {
                address: p.address,
                name: p.name,
                has_claimed_name: p.has_claimed_name,
                profile_picture_url: p.profile_picture_url,
                ..Friend::default()
            })
            .collect())
    }

    async fn subscribe_connectivity(&self) -> Result<UnboundedReceiver<Connectivity>, SocialError> {
        use crate::proto::social_service::v2::ConnectivityStatus;
        let mut updates = tokio::time::timeout(
            SUBSCRIBE_TIMEOUT,
            self.0.subscribe_to_friend_connectivity_updates(),
        )
        .await
        .map_err(|_| SocialError::request("timeout subscribing to connectivity updates"))?
        .map_err(|e| rpc("subscribe to connectivity updates", e))?;

        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        tokio::spawn(async move {
            while let Some(update) = updates.recv().await {
                let Some(friend) = update.friend else {
                    continue;
                };
                let online = update.status != ConnectivityStatus::Offline as i32;
                let update = Connectivity {
                    address: friend.address,
                    online,
                };
                if tx.send(update).is_err() {
                    break;
                }
            }
        });
        Ok(rx)
    }

    async fn subscribe(&self) -> Result<UnboundedReceiver<()>, SocialError> {
        let mut updates =
            tokio::time::timeout(SUBSCRIBE_TIMEOUT, self.0.subscribe_to_friendship_updates())
                .await
                .map_err(|_| SocialError::request("timeout subscribing to friendship updates"))?
                .map_err(|e| rpc("subscribe to friendship updates", e))?;

        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        tokio::spawn(async move {
            while let Some(update) = updates.recv().await {
                if changes_lists(&update) && tx.send(()).is_err() {
                    break;
                }
            }
        });
        Ok(rx)
    }
}

/// Block updates arrive on this stream too but do not touch the friend lists.
fn changes_lists(update: &crate::proto::social_service::v2::FriendshipUpdate) -> bool {
    !matches!(
        update.update,
        None | Some(friendship_update::Update::Block(_))
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proto::social_service::v2::{
        FriendshipRequestResponse, FriendshipRequests, FriendshipUpdate,
        PaginatedFriendshipRequestsResponse, UpsertFriendshipResponse,
    };
    use crate::proto::social_service::InternalServerError;

    fn profile(address: &str) -> FriendProfile {
        FriendProfile {
            address: address.into(),
            name: "n".into(),
            has_claimed_name: true,
            profile_picture_url: "u".into(),
            name_color: None,
        }
    }

    #[test]
    fn pagination_zero_limit_is_unpaged() {
        assert_eq!(pagination(0, 5), None);
        assert_eq!(
            pagination(100, 200),
            Some(Pagination {
                limit: 100,
                offset: 200
            })
        );
    }

    #[test]
    fn requests_carry_the_friendship_id_and_skip_empty_friends() {
        let response = PaginatedFriendshipRequestsResponse {
            response: Some(paginated_friendship_requests_response::Response::Requests(
                FriendshipRequests {
                    requests: vec![
                        FriendshipRequestResponse {
                            friend: Some(profile("0xa")),
                            created_at: 1,
                            message: None,
                            id: "id-1".into(),
                        },
                        FriendshipRequestResponse {
                            friend: None,
                            created_at: 2,
                            message: None,
                            id: "id-2".into(),
                        },
                    ],
                },
            )),
            pagination_data: None,
        };
        let list = requests(response).unwrap();
        assert_eq!(
            list,
            vec![Friend {
                address: "0xa".into(),
                name: "n".into(),
                has_claimed_name: true,
                profile_picture_url: "u".into(),
                friendship_id: "id-1".into(),
                online: false,
            }]
        );
    }

    #[test]
    fn a_server_error_in_the_request_list_is_an_error() {
        let response = PaginatedFriendshipRequestsResponse {
            response: Some(
                paginated_friendship_requests_response::Response::InternalServerError(
                    InternalServerError {
                        message: Some("boom".into()),
                    },
                ),
            ),
            pagination_data: None,
        };
        assert_eq!(requests(response), Err(SocialError::request("boom")));
    }

    #[test]
    fn upsert_rejections_are_errors() {
        let ok = UpsertFriendshipResponse {
            response: Some(upsert_friendship_response::Response::Accepted(
                upsert_friendship_response::Accepted {
                    id: "x".into(),
                    created_at: 0,
                    friend: None,
                    message: None,
                },
            )),
        };
        assert_eq!(upsert(ok), Ok(()));
        let bad = UpsertFriendshipResponse {
            response: Some(
                upsert_friendship_response::Response::InvalidFriendshipAction(
                    crate::proto::social_service::InvalidFriendshipAction { message: None },
                ),
            ),
        };
        assert_eq!(
            upsert(bad),
            Err(SocialError::request("invalid friendship action"))
        );
        assert!(upsert(UpsertFriendshipResponse { response: None }).is_err());
    }

    #[test]
    fn block_updates_do_not_change_the_lists() {
        assert!(!changes_lists(&FriendshipUpdate { update: None }));
    }
}
