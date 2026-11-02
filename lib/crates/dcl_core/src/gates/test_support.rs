//! A scripted `Fetch` for the gates' tests.

use std::{collections::HashMap, sync::Mutex};

use async_trait::async_trait;
use serde_json::Value;

use super::fetch::{Fetch, SignedFetchError};

type Reply = Result<Value, String>;

#[derive(Default)]
pub struct FakeFetch {
    any_get: Option<Reply>,
    get_routes: HashMap<String, Reply>,
    post_routes: HashMap<String, Reply>,
    signed: Option<Result<(), SignedFetchError>>,
    hang: bool,
    gets: Mutex<Vec<String>>,
    posts: Mutex<Vec<(String, String)>>,
    signed_calls: Mutex<Vec<(String, String)>>,
}

impl FakeFetch {
    /// Answers every GET with `reply`.
    pub fn with_get(reply: Reply) -> Self {
        Self {
            any_get: Some(reply),
            ..Default::default()
        }
    }

    /// Never answers.
    pub fn hanging() -> Self {
        Self {
            hang: true,
            ..Default::default()
        }
    }

    pub fn get(mut self, url: &str, reply: Reply) -> Self {
        self.get_routes.insert(url.to_string(), reply);
        self
    }

    pub fn post(mut self, url: &str, reply: Reply) -> Self {
        self.post_routes.insert(url.to_string(), reply);
        self
    }

    pub fn signed(mut self, reply: Result<(), SignedFetchError>) -> Self {
        self.signed = Some(reply);
        self
    }

    pub fn gets(&self) -> Vec<String> {
        self.gets.lock().unwrap().clone()
    }

    pub fn posts(&self) -> Vec<(String, String)> {
        self.posts.lock().unwrap().clone()
    }

    pub fn signed_calls(&self) -> Vec<(String, String)> {
        self.signed_calls.lock().unwrap().clone()
    }
}

#[async_trait]
impl Fetch for FakeFetch {
    async fn get_json(&self, url: &str) -> Reply {
        self.gets.lock().unwrap().push(url.to_string());
        if self.hang {
            std::future::pending::<()>().await;
        }
        self.get_routes
            .get(url)
            .or(self.any_get.as_ref())
            .cloned()
            .unwrap_or_else(|| Err(format!("no route: GET {url}")))
    }

    async fn post_json(&self, url: &str, body: String) -> Reply {
        self.posts.lock().unwrap().push((url.to_string(), body));
        if self.hang {
            std::future::pending::<()>().await;
        }
        self.post_routes
            .get(url)
            .cloned()
            .unwrap_or_else(|| Err(format!("no route: POST {url}")))
    }

    async fn signed_post(&self, url: &str, metadata: &str) -> Result<(), SignedFetchError> {
        self.signed_calls
            .lock()
            .unwrap()
            .push((url.to_string(), metadata.to_string()));
        if self.hang {
            std::future::pending::<()>().await;
        }
        self.signed
            .clone()
            .unwrap_or_else(|| Err(SignedFetchError::Rejected("no route".into())))
    }
}
