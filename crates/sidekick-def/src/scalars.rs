//! The small types a definition is written in: addresses, byte strings, one-or-many lists.

use schemars::{json_schema, JsonSchema, Schema, SchemaGenerator};
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use std::{borrow::Cow, fmt};

/// A 16-bit address, as the processor sees it. TOML writes it as an integer (`0x8F21`); JSON,
/// which has no hex numbers, as a number or a string (`"0x8F21"`). It's written back as such a
/// string.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Address(pub u16);

impl Serialize for Address {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&format!("0x{:04X}", self.0))
    }
}

impl<'de> Deserialize<'de> for Address {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Visitor;

        impl de::Visitor<'_> for Visitor {
            type Value = Address;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("an address from 0 to 0xFFFF, as a number or a \"0x…\" string")
            }

            fn visit_u64<E: de::Error>(self, v: u64) -> Result<Address, E> {
                u16::try_from(v)
                    .map(Address)
                    .map_err(|_| E::custom(format!("{v} is past 0xFFFF")))
            }

            fn visit_i64<E: de::Error>(self, v: i64) -> Result<Address, E> {
                u64::try_from(v)
                    .map_err(|_| E::custom(format!("{v} is below 0")))
                    .and_then(|v| self.visit_u64(v))
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<Address, E> {
                let parsed = v
                    .strip_prefix("0x")
                    .or_else(|| v.strip_prefix("0X"))
                    .map_or_else(|| v.parse(), |hex| u16::from_str_radix(hex, 16));
                parsed
                    .map(Address)
                    .map_err(|_| E::custom(format!("{v:?} isn't an address from 0 to 0xFFFF")))
            }
        }

        deserializer.deserialize_any(Visitor)
    }
}

impl JsonSchema for Address {
    fn schema_name() -> Cow<'static, str> {
        "Address".into()
    }

    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "description": "A 16-bit address: a number from 0 to 65535, or a \"0x\" hex string.",
            "oneOf": [
                { "type": "integer", "minimum": 0, "maximum": 65535 },
                { "type": "string", "pattern": "^(0[xX][0-9A-Fa-f]{1,4}|[0-9]{1,5})$" }
            ]
        })
    }
}

/// Bytes written as hex, separated by spaces: `"CD 40 75"`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bytes(pub Vec<u8>);

impl Serialize for Bytes {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let hex: Vec<String> = self.0.iter().map(|b| format!("{b:02X}")).collect();
        serializer.serialize_str(&hex.join(" "))
    }
}

impl<'de> Deserialize<'de> for Bytes {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        text.split_whitespace()
            .map(|byte| {
                u8::from_str_radix(byte, 16)
                    .ok()
                    .filter(|_| byte.len() == 2)
                    .ok_or_else(|| {
                        de::Error::custom(format!("{byte:?} isn't a hex byte like \"C9\""))
                    })
            })
            .collect::<Result<_, _>>()
            .map(Bytes)
    }
}

impl JsonSchema for Bytes {
    fn schema_name() -> Cow<'static, str> {
        "Bytes".into()
    }

    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "description": "Bytes in hex, separated by spaces: \"CD 40 75\".",
            "type": "string",
            "pattern": "^\\s*([0-9A-Fa-f]{2}(\\s+[0-9A-Fa-f]{2})*)?\\s*$"
        })
    }
}

/// One of something, or a list of them.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum OneOrMany<T> {
    /// Just one.
    One(T),
    /// A list.
    Many(Vec<T>),
}

impl<'a, T> IntoIterator for &'a OneOrMany<T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<T> OneOrMany<T> {
    /// Each of them, in order.
    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        match self {
            Self::One(one) => std::slice::from_ref(one).iter(),
            Self::Many(many) => many.iter(),
        }
    }
}
