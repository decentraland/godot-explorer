use dcl_core::{flags, urls, Core};
use godot::classes::Json;
use godot::prelude::*;

use super::{dcl_global::DclGlobal, promise::Promise};

/// Static access to the core's remote feature flags (`dcl_core::flags`).
#[derive(GodotClass)]
#[class(init, base=RefCounted)]
pub struct DclFeatureFlags {
    _base: Base<RefCounted>,
}

fn to_dictionary(flags: serde_json::Map<String, serde_json::Value>) -> VarDictionary {
    let json = serde_json::Value::Object(flags).to_string();
    Json::parse_string(&json)
        .try_to::<VarDictionary>()
        .unwrap_or_default()
}

#[godot_api]
impl DclFeatureFlags {
    /// Fetches the flags (5 s budget). Resolves with the flags dictionary, rejects with the
    /// reason on failure; either way the flags count as loaded afterwards.
    #[func]
    pub fn async_load() -> Gd<Promise> {
        let (promise, get_promise) = Promise::make_to_async();
        let global = DclGlobal::singleton();
        let requester = global
            .bind()
            .get_http_requester()
            .bind()
            .get_http_queue_requester();

        Core::spawn(async move {
            let result = flags::fetch(&requester, &urls::feature_flags()).await;
            let Some(mut promise) = get_promise() else {
                return;
            };
            match result {
                Ok(()) => promise
                    .bind_mut()
                    .resolve_with_data(to_dictionary(flags::all()).to_variant()),
                Err(reason) => promise.bind_mut().reject(GString::from(&reason)),
            }
        });
        promise
    }

    #[func]
    pub fn is_loaded() -> bool {
        flags::is_loaded()
    }

    #[func]
    pub fn is_enabled(name: GString, default_value: bool) -> bool {
        flags::is_enabled(&name.to_string(), default_value)
    }

    #[func]
    pub fn get_number(name: GString, default_value: f64) -> f64 {
        flags::get_number(&name.to_string(), default_value)
    }

    #[func]
    pub fn flags() -> VarDictionary {
        to_dictionary(flags::all())
    }

    /// The flags object of a mobile-bff response, empty on any shape mismatch.
    #[func]
    pub fn parse_response(json: Variant) -> VarDictionary {
        let value = serde_json::from_str(&Json::stringify(&json).to_string())
            .unwrap_or(serde_json::Value::Null);
        to_dictionary(flags::parse_response(&value))
    }
}
