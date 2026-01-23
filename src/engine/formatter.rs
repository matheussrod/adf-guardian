use serde_json::Value;

/// Formats the `actual_value` for display based on the guard type.
/// For most guards, it stringifies the JSON value. For `Count`, it returns the array length.
pub fn format_actual_value(guard: &str, actual_value: &Value) -> String {
    match guard {
        "Count" => {
            if let Some(arr) = actual_value.as_array() {
                arr.len().to_string()
            } else {
                // Fallback for non-array values, though `check_count` should prevent this path on success.
                "0".to_string()
            }
        }
        // Default behavior for all other guards
        _ => actual_value.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_format_count_guard() {
        let value_empty = json!([]);
        let value_three = json!([1, 2, 3]);
        assert_eq!(format_actual_value("Count", &value_empty), "0");
        assert_eq!(format_actual_value("Count", &value_three), "3");
    }

    #[test]
    fn test_format_count_guard_with_non_array() {
        let value = json!("not an array");
        assert_eq!(format_actual_value("Count", &value), "0");
    }

    #[test]
    fn test_format_default_guards() {
        let value_str = json!("a_string");
        let value_num = json!(123);
        let value_obj = json!({ "key": "value" });

        assert_eq!(
            format_actual_value("PatternMatch", &value_str),
            "\"a_string\""
        );

        assert_eq!(
            format_actual_value("AllowedValues", &value_str),
            "\"a_string\""
        );

        assert_eq!(format_actual_value("UnknownGuard", &value_num), "123");
        assert_eq!(
            format_actual_value("Exists", &value_obj),
            "{\"key\":\"value\"}"
        );
    }
}
