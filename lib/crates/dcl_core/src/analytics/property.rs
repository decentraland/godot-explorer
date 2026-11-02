//! Typed event properties: what the shell passes instead of a JSON string (D-5).

use serde_json::{Map, Value};

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Enum))]
pub enum PropertyValue {
    Text { value: String },
    Int { value: i64 },
    Float { value: f64 },
    Bool { value: bool },
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct Property {
    pub key: String,
    pub value: PropertyValue,
}

impl PropertyValue {
    /// A non-finite float has no JSON form and becomes `null`.
    fn to_json(&self) -> Value {
        match self {
            Self::Text { value } => Value::String(value.clone()),
            Self::Int { value } => Value::from(*value),
            Self::Float { value } => serde_json::Number::from_f64(*value)
                .map(Value::Number)
                .unwrap_or(Value::Null),
            Self::Bool { value } => Value::Bool(*value),
        }
    }
}

/// The properties as a JSON object; the last of two equal keys wins and empty keys are dropped.
pub fn to_object(properties: &[Property]) -> Map<String, Value> {
    properties
        .iter()
        .filter(|p| !p.key.is_empty())
        .map(|p| (p.key.clone(), p.value.to_json()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prop(key: &str, value: PropertyValue) -> Property {
        Property {
            key: key.into(),
            value,
        }
    }

    #[test]
    fn every_kind_keeps_its_json_type() {
        let object = to_object(&[
            prop("s", PropertyValue::Text { value: "x".into() }),
            prop("i", PropertyValue::Int { value: -3 }),
            prop("f", PropertyValue::Float { value: 1.5 }),
            prop("b", PropertyValue::Bool { value: true }),
        ]);
        assert_eq!(object["s"], "x");
        assert_eq!(object["i"], -3);
        assert_eq!(object["f"], 1.5);
        assert_eq!(object["b"], true);
    }

    #[test]
    fn non_finite_floats_are_null_and_empty_keys_dropped() {
        let object = to_object(&[
            prop("nan", PropertyValue::Float { value: f64::NAN }),
            prop("", PropertyValue::Bool { value: true }),
        ]);
        assert!(object["nan"].is_null());
        assert_eq!(object.len(), 1);
    }

    #[test]
    fn the_last_equal_key_wins() {
        let object = to_object(&[
            prop("k", PropertyValue::Int { value: 1 }),
            prop("k", PropertyValue::Int { value: 2 }),
        ]);
        assert_eq!(object["k"], 2);
    }
}
