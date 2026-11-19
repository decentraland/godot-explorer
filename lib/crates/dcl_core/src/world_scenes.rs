//! A world's whole `/scenes` listing. The Worlds server returns at most one page of scenes per
//! request and says how many there are in `total`; reading only the first page loses every scene
//! past it.

use std::future::Future;

use serde_json::{json, Value};

use crate::realm::dcl_world_url;

/// The most scenes the Worlds server returns per request.
const PAGE_SIZE: usize = 100;
/// A server that keeps answering without reaching `total` still ends.
const MAX_PAGES: usize = 50;

/// Every scene of `dcl_name`, as one `{"scenes": [...], "total": n}` listing. `get` fetches one
/// page (None on failure). None when even the first page fails; a later failure keeps what came.
pub async fn world_scene_listing<F, Fut>(dcl_name: &str, get: F) -> Option<Value>
where
    F: Fn(String) -> Fut,
    Fut: Future<Output = Option<Value>>,
{
    let base = format!("{}/scenes", dcl_world_url(dcl_name));
    let mut scenes: Vec<Value> = Vec::new();
    for page_index in 0..MAX_PAGES {
        let url = format!("{base}?limit={PAGE_SIZE}&offset={}", scenes.len());
        let page = match get(url).await {
            Some(page) => page,
            None if page_index == 0 => return None,
            None => break,
        };
        let rows = page
            .get("scenes")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let total = page.get("total").and_then(Value::as_u64);
        let read = rows.len();
        scenes.extend(rows);
        if read == 0 || total.is_none_or(|total| scenes.len() as u64 >= total) {
            break;
        }
    }
    let total = scenes.len();
    Some(json!({ "scenes": scenes, "total": total }))
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;

    fn scene(parcel: &str) -> Value {
        json!({ "parcels": [parcel], "entity": { "pointers": [parcel] } })
    }

    /// Reads a Worlds server holding `count` scenes, `page` per request, failing from request
    /// `fail_from` on; returns the listing and the URLs asked.
    async fn read(
        count: usize,
        page: usize,
        fail_from: Option<usize>,
    ) -> (Option<Value>, Vec<String>) {
        let all: Vec<Value> = (0..count).map(|i| scene(&format!("{i},0"))).collect();
        let asked = RefCell::new(Vec::new());
        let listing = world_scene_listing("qa.dcl.eth", |url: String| {
            asked.borrow_mut().push(url.clone());
            let request = asked.borrow().len() - 1;
            let offset: usize = url.rsplit("offset=").next().unwrap().parse().unwrap();
            let rows: Vec<Value> = all.iter().skip(offset).take(page).cloned().collect();
            let failed = fail_from.is_some_and(|from| request >= from);
            async move { (!failed).then(|| json!({ "scenes": rows, "total": count })) }
        })
        .await;
        (listing, asked.into_inner())
    }

    fn parcels(listing: &Value) -> Vec<String> {
        listing["scenes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|scene| scene["parcels"][0].as_str().unwrap().to_string())
            .collect()
    }

    #[tokio::test]
    async fn one_page_is_one_request() {
        let (listing, asked) = read(21, 100, None).await;
        assert_eq!(parcels(&listing.unwrap()).len(), 21);
        assert_eq!(asked.len(), 1);
        assert!(
            asked[0].ends_with("/qa.dcl.eth/scenes?limit=100&offset=0"),
            "{}",
            asked[0]
        );
    }

    #[tokio::test]
    async fn every_page_is_read_until_the_total() {
        let (listing, asked) = read(127, 100, None).await;
        let listing = listing.unwrap();
        assert_eq!(listing["total"], 127);
        assert_eq!(parcels(&listing).last().unwrap(), "126,0");
        assert_eq!(asked.len(), 2);
        assert!(asked[1].ends_with("offset=100"), "{}", asked[1]);
    }

    #[tokio::test]
    async fn a_failing_later_page_keeps_what_came() {
        let (listing, _) = read(127, 100, Some(1)).await;
        assert_eq!(parcels(&listing.unwrap()).len(), 100);
    }

    #[tokio::test]
    async fn a_failing_first_page_is_no_listing() {
        let (listing, _) = read(127, 100, Some(0)).await;
        assert!(listing.is_none());
    }
}
