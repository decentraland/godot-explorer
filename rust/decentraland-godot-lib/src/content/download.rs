use crate::http_request::request_response::{RequestOption, ResponseType};

use super::{content_notificator::ContentState, content_provider::ContentProviderContext};

pub async fn fetch_resource_or_wait(
    url: &String,
    file_hash: &String,
    absolute_file_path: &String,
    ctx: ContentProviderContext,
) -> Result<(), String> {
    let content_state = ctx
        .content_notificator
        .get_or_create_notify(file_hash)
        .await;

    match content_state {
        ContentState::Busy(notify) => {
            notify.notified().await;
            match ctx.content_notificator.get(file_hash).await {
                Some(ContentState::Released(result)) => result,
                _ => Err("Double busy state ".to_string()),
            }
        }
        ContentState::Released(result) => result,
        ContentState::RequestOwner => {
            #[cfg(not(target_arch = "wasm32"))]
            if tokio::fs::metadata(&absolute_file_path).await.is_err() {
                let request = RequestOption::new(
                    0,
                    url.clone(),
                    http::Method::GET,
                    ResponseType::ToFile(absolute_file_path.clone()),
                    None,
                    None,
                    None,
                );

                let result = match ctx.http_queue_requester.request(request, 0).await {
                    Ok(_response) => Ok(()),
                    Err(err) => Err(format!(
                        "Error downloading content {url} ({absolute_file_path}): {:?}",
                        err
                    )),
                };

                ctx.content_notificator
                    .resolve(file_hash, result.clone())
                    .await;
                result
            } else {
                ctx.content_notificator.resolve(file_hash, Ok(())).await;
                Ok(())
            }
            Err("Wasm32 not supported".to_string())
        }
    }
}
