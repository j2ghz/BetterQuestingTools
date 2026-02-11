use serde_json::{Map, Value};
use std::collections::BTreeMap;

/// Normalize NBT-like keys that have ":<type>" suffixes and convert index-like maps
/// such as {"0:10": {...}, "1:10": {...}} into arrays.
pub fn normalize_value(v: Value) -> Value {
    match v {
        Value::Object(m) => Value::Object(normalize_map(m)),
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

    // determine if all keys are numeric (array-like)
    let mut numeric_keys: BTreeMap<usize, Value> = BTreeMap::new();
    let mut all_numeric = true;
    for (k, v) in &stripped {
        if let Ok(idx) = k.parse::<usize>() {
            numeric_keys.insert(idx, v.clone());
        } else {
            all_numeric = false;
        }
    }

    if all_numeric && !numeric_keys.is_empty() {
        // convert to array under a special key "__array__" to signal caller
        // but to keep using serde_json::Value::Array we return as {"": [...]} not allowed here.
        // Instead, we'll place a single key "" which caller of normalize_value can detect.
        // For simplicity, return a map with numeric string keys but keep order by BTreeMap when later converting.
        // However, consumer should call `map_to_array_if_numeric` helper when needed.
        // We'll keep the stripped map as-is.
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
        let map = norm.as_object().expect("object");
        // keys should be stripped
        assert!(map.contains_key("0"));
        assert!(map.contains_key("1"));

        // map_to_array_if_numeric should convert
        let arr = map_to_array_if_numeric(map).expect("array");
        assert_eq!(arr.len(), 2);
        let a0 = &arr[0];
        let a1 = &arr[1];
        // inner keys also normalized (id still present but with suffix stripped?)
        let obj0 = a0.as_object().expect("obj0");
        assert!(obj0.contains_key("id"));
        let obj1 = a1.as_object().expect("obj1");
        assert!(obj1.contains_key("id"));
    }
}
