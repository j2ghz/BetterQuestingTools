use serde_json::{Map, Value};
use std::collections::BTreeMap;

/// Normalize NBT-like keys that have ":<type>" suffixes and convert index-like maps
/// such as {"0:10": {...}, "1:10": {...}} into arrays.
pub fn normalize_value(v: Value) -> Value {
    match v {
        Value::Object(m) => {
            let stripped = normalize_map(m);
            // if all keys are numeric, convert to array
            if let Some(arr) = map_to_array_if_numeric(&stripped) {
                Value::Array(arr.into_iter().map(normalize_value).collect())
            } else {
                Value::Object(stripped)
            }
        }
        Value::Array(a) => Value::Array(a.into_iter().map(normalize_value).collect()),
        other => other,
    }
}

fn normalize_map(m: Map<String, Value>) -> Map<String, Value> {
    // first, strip suffixes from keys
    let mut stripped: Map<String, Value> = Map::new();
    for (k, v) in m {
        let key = match k.rfind(':') {
            Some(pos) => k[..pos].to_string(),
            None => k,
        };
        stripped.insert(key, normalize_value(v));
    }

    stripped
}

/// Helper to convert a serde_json::Map whose keys are numeric indices into a Vec<Value>.
pub fn map_to_array_if_numeric(m: &Map<String, Value>) -> Option<Vec<Value>> {
    let mut numeric_keys: BTreeMap<usize, Value> = BTreeMap::new();
    for (k, v) in m {
        if let Ok(idx) = k.parse::<usize>() {
            numeric_keys.insert(idx, v.clone());
        } else {
            return None;
        }
    }
    if numeric_keys.is_empty() {
        return None;
    }
    Some(numeric_keys.into_iter().map(|(_, v)| v).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn strip_suffix_and_array_conversion() {
        let v = json!({ "0:10": { "id:8": "foo" }, "1:10": { "id:8": "bar" } });
        let norm = normalize_value(v);
        // normalization should convert top-level numeric-keyed map into an array
        if let Some(arr) = norm.as_array() {
            assert_eq!(arr.len(), 2);
            let a0 = &arr[0];
            let a1 = &arr[1];
            let obj0 = a0.as_object().expect("obj0");
            let obj1 = a1.as_object().expect("obj1");
            assert!(obj0.contains_key("id"));
            assert!(obj1.contains_key("id"));
        } else {
            panic!("expected array after normalization");
        }
    }
}
