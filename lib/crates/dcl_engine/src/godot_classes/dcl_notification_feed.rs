use dcl_core::notifications::{self, Feed};
use godot::prelude::*;
use serde_json::{Map, Number, Value};

/// The in-app notification feed (`dcl_core::notifications::Feed`) for the GDScript manager,
/// which still fetches and renders. Integers stay integers across the boundary.
#[derive(GodotClass)]
#[class(init, base=RefCounted)]
pub struct DclNotificationFeed {
    feed: Feed,
    base: Base<RefCounted>,
}

fn to_value(variant: &Variant) -> Value {
    match variant.get_type() {
        VariantType::BOOL => Value::Bool(variant.to::<bool>()),
        VariantType::INT => Value::Number(variant.to::<i64>().into()),
        VariantType::FLOAT => {
            Number::from_f64(variant.to::<f64>()).map_or(Value::Null, Value::Number)
        }
        VariantType::STRING | VariantType::STRING_NAME => Value::String(variant.to_string()),
        VariantType::DICTIONARY => {
            let mut object = Map::new();
            for (key, value) in variant.to::<VarDictionary>().iter_shared() {
                object.insert(key.to_string(), to_value(&value));
            }
            Value::Object(object)
        }
        VariantType::ARRAY => Value::Array(
            variant
                .to::<VarArray>()
                .iter_shared()
                .map(|item| to_value(&item))
                .collect(),
        ),
        VariantType::NIL => Value::Null,
        _ => Value::String(variant.to_string()),
    }
}

fn from_value(value: &Value) -> Variant {
    match value {
        Value::Null => Variant::nil(),
        Value::Bool(b) => b.to_variant(),
        Value::Number(n) => match n.as_i64() {
            Some(int) => int.to_variant(),
            None => n.as_f64().unwrap_or_default().to_variant(),
        },
        Value::String(text) => text.as_str().to_variant(),
        Value::Array(items) => items
            .iter()
            .map(from_value)
            .collect::<VarArray>()
            .to_variant(),
        Value::Object(object) => {
            let mut dict = VarDictionary::new();
            for (key, value) in object {
                dict.set(key.as_str(), from_value(value));
            }
            dict.to_variant()
        }
    }
}

fn values(items: &VarArray) -> Vec<Value> {
    items.iter_shared().map(|item| to_value(&item)).collect()
}

fn array(items: &[Value]) -> VarArray {
    items.iter().map(from_value).collect()
}

#[godot_api]
impl DclNotificationFeed {
    /// Keeps the supported notification types; `is_blocked(address) -> bool` vets friend-request
    /// senders.
    #[func]
    pub fn filter(&self, items: VarArray, filter_enabled: bool, is_blocked: Callable) -> VarArray {
        let blocked = |address: &str| {
            is_blocked
                .call(&[address.to_variant()])
                .try_to::<bool>()
                .unwrap_or(false)
        };
        array(&notifications::filter(
            values(&items),
            filter_enabled,
            &blocked,
        ))
    }

    /// Merges filtered notifications and queues the new unread, recent ones for toasts. Returns
    /// `{ queued: int, raise: Dictionary | null }`; `raise` is the toast to show now, set only
    /// when the queue was empty before.
    #[func]
    pub fn ingest(&mut self, items: VarArray, now_ms: i64) -> VarDictionary {
        let outcome = self.feed.ingest(values(&items), now_ms);
        let mut dict = VarDictionary::new();
        dict.set("queued", outcome.queued as i64);
        dict.set(
            "raise",
            outcome.raise.as_ref().map(from_value).unwrap_or_default(),
        );
        dict
    }

    #[func]
    pub fn mark_read(&mut self, ids: PackedStringArray) {
        let ids: Vec<String> = ids.as_slice().iter().map(|id| id.to_string()).collect();
        self.feed.mark_read(&ids);
    }

    #[func]
    pub fn get_all(&self) -> VarArray {
        array(self.feed.all())
    }

    /// Newest first.
    #[func]
    pub fn get_sorted(&self) -> VarArray {
        array(&self.feed.sorted())
    }

    #[func]
    pub fn clear(&mut self) {
        self.feed.clear();
    }

    /// Appends a toast; returns the queue length afterwards.
    #[func]
    pub fn enqueue(&mut self, notification: VarDictionary) -> i64 {
        self.feed.enqueue(to_value(&notification.to_variant())) as i64
    }

    /// The toast at the head of the queue, or an empty dictionary.
    #[func]
    pub fn peek(&self) -> VarDictionary {
        self.feed
            .peek()
            .and_then(|head| from_value(head).try_to::<VarDictionary>().ok())
            .unwrap_or_default()
    }

    /// Drops the head of the queue.
    #[func]
    pub fn pop(&mut self) {
        self.feed.pop();
    }

    #[func]
    pub fn queue_size(&self) -> i64 {
        self.feed.queue_len() as i64
    }

    #[func]
    pub fn clear_system_toasts(&mut self) {
        self.feed.clear_system_toasts();
    }
}
