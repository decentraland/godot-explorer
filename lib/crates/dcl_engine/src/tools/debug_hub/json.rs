//! Variants as JSON, for the properties a debug client asks for by name.

use godot::builtin::{Aabb, Transform3D};
use godot::prelude::*;
use serde_json::{json, Value};

pub fn vec3(v: Vector3) -> Value {
    json!([v.x, v.y, v.z])
}

pub fn vec2(v: Vector2) -> Value {
    json!([v.x, v.y])
}

pub fn transform(t: Transform3D) -> Value {
    json!({
        "position": vec3(t.origin),
        "rotation_euler": vec3(t.basis.get_euler()),
        "scale": vec3(t.basis.get_scale()),
    })
}

pub fn aabb(a: Aabb) -> Value {
    json!({"position": vec3(a.position), "size": vec3(a.size)})
}

/// A JSON-safe rendering of `value`: structured for the common math types, `str()` for anything
/// else (object references, node paths, callables, RIDs).
pub fn variant_to_json(value: &Variant) -> Value {
    match value.get_type() {
        VariantType::NIL => Value::Null,
        VariantType::BOOL => json!(value.to::<bool>()),
        VariantType::INT => json!(value.to::<i64>()),
        VariantType::FLOAT => json!(value.to::<f64>()),
        VariantType::STRING | VariantType::STRING_NAME => json!(value.stringify().to_string()),
        VariantType::VECTOR2 => vec2(value.to::<Vector2>()),
        VariantType::VECTOR2I => {
            let v = value.to::<Vector2i>();
            json!([v.x, v.y])
        }
        VariantType::VECTOR3 => vec3(value.to::<Vector3>()),
        VariantType::VECTOR3I => {
            let v = value.to::<Vector3i>();
            json!([v.x, v.y, v.z])
        }
        VariantType::VECTOR4 => {
            let v = value.to::<Vector4>();
            json!([v.x, v.y, v.z, v.w])
        }
        VariantType::QUATERNION => {
            let q = value.to::<Quaternion>();
            json!([q.x, q.y, q.z, q.w])
        }
        VariantType::COLOR => {
            let c = value.to::<Color>();
            json!([c.r, c.g, c.b, c.a])
        }
        VariantType::AABB => aabb(value.to::<Aabb>()),
        VariantType::RECT2 => {
            let r = value.to::<Rect2>();
            json!({"position": vec2(r.position), "size": vec2(r.size)})
        }
        VariantType::RECT2I => {
            let r = value.to::<Rect2i>();
            json!({"position": [r.position.x, r.position.y], "size": [r.size.x, r.size.y]})
        }
        VariantType::TRANSFORM3D => transform(value.to::<Transform3D>()),
        VariantType::ARRAY => Value::Array(
            value
                .to::<VarArray>()
                .iter_shared()
                .map(|item| variant_to_json(&item))
                .collect(),
        ),
        VariantType::PACKED_INT32_ARRAY => json!(value.to::<PackedInt32Array>().to_vec()),
        VariantType::PACKED_INT64_ARRAY => json!(value.to::<PackedInt64Array>().to_vec()),
        VariantType::DICTIONARY => Value::Object(
            value
                .to::<VarDictionary>()
                .iter_shared()
                .map(|(key, item)| (key.stringify().to_string(), variant_to_json(&item)))
                .collect(),
        ),
        _ => json!(value.stringify().to_string()),
    }
}

/// `object.get(name)` as JSON.
pub fn property(object: &Gd<impl Inherits<Object>>, name: &str) -> Value {
    variant_to_json(&object.clone().upcast::<Object>().get(name))
}
