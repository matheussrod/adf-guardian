use regex::Regex;
use serde_json::Value;

pub fn check_pattern_match(node: &Value, params: &Value) -> bool {
    let regex_str = params.get("regex").and_then(|v| v.as_str());
    let negative = params
        .get("negative")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    if let (Some(text), Some(pattern)) = (node.as_str(), regex_str) {
        if let Ok(re) = Regex::new(pattern) {
            let is_match = re.is_match(text);
            if negative { !is_match } else { is_match }
        } else {
            false
        }
    } else {
        false
    }
}

pub fn check_allowed_values(node: &Value, params: &Value) -> bool {
    let values = params.get("values").and_then(|v| v.as_array());
    let mode = params
        .get("mode")
        .and_then(|v| v.as_str())
        .unwrap_or("Allow"); // Allow or Deny
    let case_sensitive = params
        .get("case_sensitive")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    let node_str = node.as_str();

    if let (Some(text), Some(list)) = (node_str, values) {
        let found = list.iter().any(|v| {
            if let Some(v_str) = v.as_str() {
                if case_sensitive {
                    v_str == text
                } else {
                    v_str.eq_ignore_ascii_case(text)
                }
            } else {
                false
            }
        });

        match mode {
            "Deny" => !found,
            _ => found, // Allow
        }
    } else {
        // Assuming strict validation: false
        false
    }
}

pub fn check_exists(node: &Value, params: &Value) -> bool {
    let should_exist = params
        .get("should_exist")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    if should_exist {
        !node.is_null()
    } else {
        node.is_null()
    }
}

pub fn check_range(node: &Value, params: &Value) -> bool {
    let min = params.get("min").and_then(|v| v.as_f64());
    let max = params.get("max").and_then(|v| v.as_f64());

    if let Some(val) = node.as_f64() {
        let min_ok = min.is_none_or(|m| val >= m);
        let max_ok = max.is_none_or(|m| val <= m);
        min_ok && max_ok
    } else {
        false // Not a number
    }
}

pub fn check_count(node: &Value, params: &Value) -> bool {
    let min = params.get("min").and_then(|v| v.as_u64());
    let max = params.get("max").and_then(|v| v.as_u64());

    if let Some(arr) = node.as_array() {
        let len = arr.len() as u64;
        let min_ok = min.is_none_or(|m| len >= m);
        let max_ok = max.is_none_or(|m| len <= m);
        min_ok && max_ok
    } else {
        false // Not an array
    }
}

pub fn check_string_length(node: &Value, params: &Value) -> bool {
    let min = params.get("min").and_then(|v| v.as_u64());
    let max = params.get("max").and_then(|v| v.as_u64());

    if let Some(s) = node.as_str() {
        let len = s.chars().count() as u64;
        let min_ok = min.is_none_or(|m| len >= m);
        let max_ok = max.is_none_or(|m| len <= m);
        min_ok && max_ok
    } else {
        false // Not a string
    }
}
