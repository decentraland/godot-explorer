use serde_json::Value;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingKind {
    Bool,
    Int,
    Float,
    Text,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Enum))]
pub enum SettingValue {
    Bool(bool),
    Int(i64),
    Float(f64),
    Text(String),
}

impl SettingValue {
    /// What the shell's number may become for `kind`: an int key truncates a float, a float key
    /// widens an int, and everything else must already match. Non-finite floats never do.
    pub fn coerce(&self, kind: SettingKind) -> Option<SettingValue> {
        match (kind, self) {
            (SettingKind::Bool, Self::Bool(_)) => Some(self.clone()),
            (SettingKind::Text, Self::Text(_)) => Some(self.clone()),
            (SettingKind::Int, Self::Int(_)) => Some(self.clone()),
            (SettingKind::Int, Self::Float(f)) if f.is_finite() && f.abs() < 9.2e18 => {
                Some(Self::Int(f.trunc() as i64))
            }
            (SettingKind::Float, Self::Float(f)) if f.is_finite() => Some(self.clone()),
            (SettingKind::Float, Self::Int(i)) => Some(Self::Float(*i as f64)),
            _ => None,
        }
    }

    pub(super) fn to_json(&self) -> Value {
        match self {
            Self::Bool(b) => Value::Bool(*b),
            Self::Int(i) => Value::from(*i),
            Self::Float(f) => Value::from(*f),
            Self::Text(s) => Value::String(s.clone()),
        }
    }

    pub(super) fn from_json(value: &Value) -> Option<Self> {
        match value {
            Value::Bool(b) => Some(Self::Bool(*b)),
            Value::Number(n) => n
                .as_i64()
                .map(Self::Int)
                .or_else(|| n.as_f64().map(Self::Float)),
            Value::String(s) => Some(Self::Text(s.clone())),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use SettingKind as K;
    use SettingValue as V;

    #[test]
    fn coercion_table() {
        let t = || V::Text("x".into());
        let cases = [
            (K::Bool, V::Bool(true), Some(V::Bool(true))),
            (K::Bool, V::Int(1), None),
            (K::Bool, t(), None),
            (K::Text, t(), Some(t())),
            (K::Text, V::Bool(true), None),
            (K::Text, V::Int(1), None),
            (K::Int, V::Int(3), Some(V::Int(3))),
            (K::Int, V::Float(3.9), Some(V::Int(3))),
            (K::Int, V::Float(-3.9), Some(V::Int(-3))),
            (K::Int, V::Bool(true), None),
            (K::Int, t(), None),
            (K::Int, V::Float(f64::NAN), None),
            (K::Int, V::Float(f64::INFINITY), None),
            (K::Int, V::Float(1e19), None),
            (K::Float, V::Float(2.5), Some(V::Float(2.5))),
            (K::Float, V::Int(7), Some(V::Float(7.0))),
            (K::Float, V::Float(f64::NEG_INFINITY), None),
            (K::Float, V::Bool(false), None),
            (K::Float, t(), None),
        ];
        for (kind, input, expected) in cases {
            assert_eq!(input.coerce(kind), expected, "{input:?} as {kind:?}");
        }
    }

    #[test]
    fn json_round_trip_keeps_the_variant() {
        for v in [
            V::Bool(false),
            V::Int(-4),
            V::Float(2.5),
            V::Text("es".into()),
        ] {
            assert_eq!(V::from_json(&v.to_json()), Some(v));
        }
        assert_eq!(V::from_json(&Value::Null), None);
        assert_eq!(V::from_json(&serde_json::json!([1])), None);
    }
}
