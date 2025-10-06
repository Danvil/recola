use excess::prelude::*;
use std::collections::HashMap;

#[derive(Component)]
pub struct CustomProperties(HashMap<String, CustomPropertiesValue>);

pub enum CustomPropertiesValue {
    Bool(bool),
    Integer(i64),
    Float(f64),
    String(String),
}

impl CustomProperties {
    pub fn from_json(map: &HashMap<String, serde_json::Value>) -> Self {
        let mut out = HashMap::new();

        for (key, value) in map {
            let parsed = match value {
                serde_json::Value::Number(num) if num.is_i64() => {
                    num.as_i64().map(CustomPropertiesValue::Integer)
                }
                serde_json::Value::Number(num) if num.is_f64() => {
                    num.as_f64().map(CustomPropertiesValue::Float)
                }
                serde_json::Value::Number(_num) => {
                    todo!()
                }
                serde_json::Value::String(s) => Some(CustomPropertiesValue::String(s.clone())),
                serde_json::Value::Bool(b) => Some(CustomPropertiesValue::Bool(*b)),
                _ => {
                    todo!()
                }
            };

            if let Some(v) = parsed {
                out.insert(key.clone(), v);
            }
        }

        CustomProperties(out)
    }

    pub fn get_integer(&self, id: impl AsRef<str>) -> Option<i64> {
        match self.0.get(id.as_ref())? {
            CustomPropertiesValue::Integer(v) => Some(*v),
            _ => None,
        }
    }

    pub fn get_string_list(&self, id: impl AsRef<str>) -> Option<Vec<String>> {
        match self.0.get(id.as_ref())? {
            CustomPropertiesValue::String(v) => Some(v.split(",").map(|s| s.to_owned()).collect()),
            _ => None,
        }
    }
}
