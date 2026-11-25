use std::collections::HashSet;

use serde_json::{Map, Value};

pub fn sanitize_json_object(raw: Option<&Value>, max_entries: usize) -> Value {
    let Some(Value::Object(map)) = raw else {
        return Value::Object(Map::new());
    };
    let mut out = Map::new();
    for (k, v) in map.iter().take(max_entries) {
        if !k.trim().is_empty() && v.is_object() {
            out.insert(k.trim().to_string(), v.clone());
        }
    }
    Value::Object(out)
}

pub fn merge_json_arrays_by_key(
    cloud: &Value,
    incoming: &Value,
    key_fn: fn(&Map<String, Value>) -> String,
    max_items: usize,
) -> Value {
    let mut seen = HashSet::new();
    let mut merged = Vec::new();
    for source in [incoming, cloud] {
        if let Value::Array(items) = source {
            for item in items {
                let Some(obj) = item.as_object() else { continue };
                let key = key_fn(obj);
                if !key.is_empty() && !seen.insert(key) {
                    continue;
                }
                merged.push(item.clone());
                if merged.len() >= max_items {
                    return Value::Array(merged);
                }
            }
        }
    }
    Value::Array(merged)
}

pub fn merge_id_lists(cloud: &[i64], incoming: &[i64], max_items: usize) -> Vec<i64> {
    let mut seen = HashSet::new();
    let mut merged = Vec::new();
    for &id in cloud.iter().chain(incoming.iter()) {
        if id > 0 && seen.insert(id) {
            merged.push(id);
            if merged.len() >= max_items {
                break;
            }
        }
    }
    merged
}

pub fn merge_game_brew_progress(cloud: &Value, incoming: &Value, max_entries: usize) -> Value {
    let mut merged = match cloud {
        Value::Object(map) => map.clone(),
        _ => Map::new(),
    };
    if let Value::Object(inc_map) = incoming {
        for (code, inc_val) in inc_map {
            let Some(inc_obj) = inc_val.as_object() else { continue };
            match merged.get(code).and_then(Value::as_object) {
                None => {
                    if merged.len() < max_entries {
                        merged.insert(code.clone(), inc_val.clone());
                    }
                }
                Some(cur_obj) => {
                    let cur_stars = cur_obj.get("stars").and_then(Value::as_i64).unwrap_or(0);
                    let inc_stars = inc_obj.get("stars").and_then(Value::as_i64).unwrap_or(0);
                    let cur_score = cur_obj.get("bestScorePct").and_then(Value::as_f64).unwrap_or(0.0);
                    let inc_score = inc_obj.get("bestScorePct").and_then(Value::as_f64).unwrap_or(0.0);
                    let cur_att = cur_obj.get("attempts").and_then(Value::as_i64).unwrap_or(0);
                    let inc_att = inc_obj.get("attempts").and_then(Value::as_i64).unwrap_or(0);

                    let mut next = if inc_score >= cur_score {
                        inc_obj.clone()
                    } else {
                        cur_obj.clone()
                    };
                    next.insert("stars".to_string(), Value::from(cur_stars.max(inc_stars).clamp(0, 3)));
                    next.insert("bestScorePct".to_string(), Value::from(cur_score.max(inc_score)));
                    next.insert("attempts".to_string(), Value::from(cur_att.max(inc_att)));
                    merged.insert(code.clone(), Value::Object(next));
                }
            }
        }
    }
    Value::Object(merged)
}

pub fn history_item_key(obj: &Map<String, Value>) -> String {
    let id = obj.get("id").map(|v| v.to_string()).unwrap_or_default();
    let at = obj.get("brewedAt").and_then(Value::as_str).unwrap_or_default();
    format!("{id}_{at}")
}

pub fn journal_item_key(obj: &Map<String, Value>) -> String {
    obj.get("id")
        .or_else(|| obj.get("createdAt"))
        .map(|v| v.to_string())
        .unwrap_or_default()
}

pub fn recipe_item_key(obj: &Map<String, Value>) -> String {
    obj.get("id").map(|v| v.to_string()).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn merge_game_brew_keeps_highest_stars_and_score() {
        let cloud = json!({
            "MB-01": {"code": "MB-01", "stars": 2, "bestScorePct": 82.0, "attempts": 3},
            "LA-01": {"code": "LA-01", "stars": 3, "bestScorePct": 95.0, "attempts": 4}
        });
        let incoming = json!({
            "MB-01": {"code": "MB-01", "stars": 3, "bestScorePct": 91.5, "attempts": 2},
            "MB-02": {"code": "MB-02", "stars": 1, "bestScorePct": 70.0, "attempts": 1}
        });
        let merged = merge_game_brew_progress(&cloud, &incoming, 100);
        assert_eq!(merged["MB-01"]["stars"], 3);
        assert_eq!(merged["MB-01"]["bestScorePct"], 91.5);
        assert_eq!(merged["MB-01"]["attempts"], 3);
        assert_eq!(merged["LA-01"]["stars"], 3);
        assert_eq!(merged["MB-02"]["stars"], 1);
    }

    #[test]
    fn stale_empty_incoming_does_not_wipe_cloud_history() {
        let cloud = json!([{"id": 10, "brewedAt": "2026-11-24T10:00:00Z"}]);
        let empty = json!([]);
        let merged = merge_json_arrays_by_key(&cloud, &empty, history_item_key, 50);
        assert_eq!(merged.as_array().map(Vec::len), Some(1));
    }
}
