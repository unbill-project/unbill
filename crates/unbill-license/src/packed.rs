use crate::types::LegalDictionary;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// Internal storage format: identical text bytes have one shared table entry.
#[derive(Serialize, Deserialize)]
pub(crate) struct PackedCatalog {
    pub texts: Vec<String>,
    pub packages: Value,
}

impl PackedCatalog {
    #[allow(dead_code, reason = "packing is used by the build script and tests")]
    pub fn pack(dictionary: &LegalDictionary) -> serde_json::Result<Self> {
        let mut packages = serde_json::to_value(dictionary)?;
        let mut texts = Vec::new();
        let mut indices = BTreeMap::new();
        visit_texts(&mut packages, &mut |value| {
            let text = value
                .as_str()
                .ok_or_else(|| invalid("expected legal text"))?;
            let index = *indices.entry(text.to_owned()).or_insert_with(|| {
                let index = texts.len();
                texts.push(text.to_owned());
                index
            });
            *value = serde_json::to_value(index)?;
            Ok(())
        })?;
        Ok(Self { texts, packages })
    }

    #[allow(dead_code, reason = "unpacking is used by the runtime and tests")]
    pub fn unpack(mut self) -> serde_json::Result<LegalDictionary> {
        visit_texts(&mut self.packages, &mut |value| {
            let index = value
                .as_u64()
                .and_then(|index| usize::try_from(index).ok())
                .ok_or_else(|| invalid("invalid legal text index"))?;
            let text = self
                .texts
                .get(index)
                .ok_or_else(|| invalid("missing legal text"))?;
            *value = Value::String(text.clone());
            Ok(())
        })?;
        serde_json::from_value(self.packages)
    }
}

fn invalid(message: &str) -> serde_json::Error {
    <serde_json::Error as serde::de::Error>::custom(message)
}

fn visit_texts(
    value: &mut Value,
    transform: &mut impl FnMut(&mut Value) -> serde_json::Result<()>,
) -> serde_json::Result<()> {
    match value {
        Value::Object(object) => {
            for (key, value) in object {
                if key == "text" && (value.is_string() || value.is_number()) {
                    transform(value)?;
                } else {
                    visit_texts(value, transform)?;
                }
            }
        }
        Value::Array(values) => {
            for value in values {
                visit_texts(value, transform)?;
            }
        }
        _ => {}
    }
    Ok(())
}
