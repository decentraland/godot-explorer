//! Executes the shell's HUD controls (insets, chat, hide UI) and reports the native HUD state.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use godot::prelude::*;

use super::host::EngineHost;
use super::{api, controls};
use crate::avatars::dcl_user_profile::DclUserProfile;
use crate::engine::local_task::{await_promise, spawn_local};
use crate::fetcher::globals;
use crate::godot_classes::dcl_global::DclGlobal;
use crate::godot_classes::promise::PromiseError;

const CHAT_CACHE_MAX: usize = 200;

#[derive(Clone, Default)]
struct SenderProfile {
    name: String,
    has_claimed_name: bool,
    picture_url: String,
}

#[derive(Default)]
pub struct Hud {
    hidden: Cell<bool>,
    chat_open: Cell<bool>,
    unread: Cell<i32>,
    profiles: RefCell<HashMap<String, SenderProfile>>,
}

impl Hud {
    pub fn connect(self: &Rc<Self>, host: &Gd<EngineHost>) {
        let me = self.clone();
        DclGlobal::singleton()
            .signals()
            .on_chat_message()
            .connect_other(host, move |_, address, message, timestamp| {
                spawn_local(me.clone().report_chat(
                    address.to_string(),
                    message.to_string(),
                    timestamp,
                ));
            });
        DclGlobal::singleton()
            .signals()
            .open_backpack()
            .connect_other(host, |_, emotes| {
                globals::engine().bind().report_open_backpack(emotes);
            });
        globals::scene_runner()
            .signals()
            .loading_progress()
            .connect_other(host, |_, percent, loaded, total| {
                globals::engine().bind().report_loading_detail(
                    loaded as i64,
                    total as i64,
                    0.0,
                    percent,
                );
            });
    }

    /// Runs the control when it belongs to the HUD; false for any other.
    pub fn handle(&self, kind: &str, args: &VarDictionary) -> bool {
        let float = |key: &str| {
            args.get(key)
                .and_then(|v| v.try_to::<f64>().ok())
                .unwrap_or(0.0)
        };
        match kind {
            "set_hud_insets" => globals::engine().bind().set_hud_insets(
                float("top") as f32,
                float("right") as f32,
                float("bottom") as f32,
                float("left") as f32,
            ),
            "set_chat_width" => {
                let width = api::device_pixels_to_viewport_units(float("width") as f32);
                DclGlobal::singleton().bind_mut().chat_notifications_width = width as i32;
                controls::push_scene_interactable_area();
            }
            "chat_open" => self.set_chat_open(args.get("open").is_some_and(|v| v.booleanize())),
            "hide_ui" => {
                let hidden = args.get("hidden").is_some_and(|v| v.booleanize());
                self.hidden.set(hidden);
                controls::apply_mobile_controls_hide_ui(hidden);
                globals::engine()
                    .bind()
                    .report_ui_state(hidden, self.chat_open.get());
            }
            _ => return false,
        }
        true
    }

    pub fn reset(&self) {
        self.hidden.set(false);
        self.chat_open.set(false);
        self.unread.set(0);
    }

    fn set_chat_open(&self, open: bool) {
        if open == self.chat_open.get() {
            return;
        }
        self.chat_open.set(open);
        controls::set_chat_focus(open);
        if open {
            self.unread.set(0);
            globals::engine().bind().report_unread_chat(0);
        }
        globals::engine()
            .bind()
            .report_ui_state(self.hidden.get(), open);
    }

    async fn report_chat(self: Rc<Self>, address: String, message: String, timestamp: f64) {
        let profile = if address.starts_with("0x") {
            self.sender_profile(&address).await
        } else {
            None
        };
        if !self.chat_open.get() {
            self.unread.set(self.unread.get() + 1);
            globals::engine()
                .bind()
                .report_unread_chat(self.unread.get());
        }
        let profile_known = profile.is_some();
        let profile = profile.unwrap_or_default();
        globals::engine().bind().report_chat(
            address.as_str().into(),
            message.as_str().into(),
            timestamp,
            profile_known,
            profile.name.as_str().into(),
            profile.has_claimed_name,
            profile.picture_url.as_str().into(),
        );
    }

    async fn sender_profile(&self, address: &str) -> Option<SenderProfile> {
        if let Some(known) = self.profiles.borrow().get(address) {
            return Some(known.clone());
        }
        let promise = globals::content_provider()
            .bind_mut()
            .fetch_profile(address.into());
        let fetched = await_promise(promise).await;
        let profile = if fetched.try_to::<Gd<PromiseError>>().is_ok() {
            None
        } else {
            fetched.try_to::<Gd<DclUserProfile>>().ok()
        };
        let sender = match profile {
            Some(profile) => {
                let profile = profile.bind();
                SenderProfile {
                    name: profile.get_name().to_string(),
                    has_claimed_name: profile.has_claimed_name(),
                    picture_url: profile
                        .get_avatar()
                        .bind()
                        .get_snapshots_face_url()
                        .to_string(),
                }
            }
            None => SenderProfile {
                name: address.to_string(),
                ..Default::default()
            },
        };
        let mut profiles = self.profiles.borrow_mut();
        if profiles.len() >= CHAT_CACHE_MAX {
            profiles.clear();
        }
        profiles.insert(address.to_string(), sender.clone());
        Some(sender)
    }
}
