//! Engine side of the core's screenshot contract: the main viewport after the next draw, saved
//! as a PNG under `user://`.

use std::{path::PathBuf, sync::Arc, time::Duration};

use async_trait::async_trait;
use dcl_core::scene::{set_screenshot_provider, ScreenshotProvider};
use godot::classes::{object::ConnectFlags, Engine, ProjectSettings, RenderingServer, SceneTree};
use godot::prelude::*;
use tokio::sync::oneshot;

use super::settings_link::run_on_main;

const SCREENSHOT_PATH: &str = "user://rn_screenshot.png";

/// Frames only draw while the engine renders; without one the capture would wait forever.
const CAPTURE_TIMEOUT: Duration = Duration::from_secs(5);

type CaptureResult = Result<PathBuf, String>;

struct ViewportScreenshot;

#[async_trait]
impl ScreenshotProvider for ViewportScreenshot {
    async fn capture(&self) -> CaptureResult {
        let (sender, receiver) = oneshot::channel();
        run_on_main(move || capture_after_draw(sender));
        match tokio::time::timeout(CAPTURE_TIMEOUT, receiver).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err("the capture was dropped".to_string()),
            Err(_) => Err("no frame was drawn".to_string()),
        }
    }
}

/// Registers the screenshot provider with the core. Called once when the engine starts.
pub(super) fn register() {
    set_screenshot_provider(Some(Arc::new(ViewportScreenshot)));
}

/// Waits for the next `frame_post_draw`, then saves the viewport. Main thread.
fn capture_after_draw(sender: oneshot::Sender<CaptureResult>) {
    let mut sender = Some(sender);
    let callable = Callable::from_fn("capture_screenshot", move |_| {
        // A caller that timed out is gone: skip the PNG.
        if let Some(sender) = sender.take().filter(|sender| !sender.is_closed()) {
            let _ = sender.send(save_viewport());
        }
        Variant::nil()
    });
    RenderingServer::singleton().connect_flags(
        "frame_post_draw",
        &callable,
        ConnectFlags::ONE_SHOT,
    );
}

fn save_viewport() -> CaptureResult {
    let root = Engine::singleton()
        .get_main_loop()
        .and_then(|main_loop| main_loop.try_cast::<SceneTree>().ok())
        .and_then(|tree| tree.get_root())
        .ok_or("no root viewport")?;
    let image = root
        .get_texture()
        .and_then(|texture| texture.get_image())
        .ok_or("the viewport has no image")?;
    if image.save_png(SCREENSHOT_PATH) != godot::global::Error::OK {
        return Err(format!("cannot save {SCREENSHOT_PATH}"));
    }
    let absolute = ProjectSettings::singleton().globalize_path(SCREENSHOT_PATH);
    Ok(PathBuf::from(absolute.to_string()))
}
