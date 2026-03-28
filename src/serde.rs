//! Custom serde helpers for flexible deserialization

use serde::{Deserialize, Deserializer};

/// Deserialize a u64 from either a number or a string
pub fn flexible_u64<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringOrU64 {
        U64(u64),
        Str(String),
    }

    match StringOrU64::deserialize(deserializer)? {
        StringOrU64::U64(v) => Ok(v),
        StringOrU64::Str(s) => s.trim().parse::<u64>().map_err(serde::de::Error::custom),
    }
}

/// Deserialize an Option<u64> from either a number, string, or null
pub fn flexible_u64_opt<'de, D>(deserializer: D) -> Result<Option<u64>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringOrU64 {
        U64(u64),
        Str(String),
    }

    let opt: Option<StringOrU64> = Option::deserialize(deserializer)?;
    match opt {
        None => Ok(None),
        Some(StringOrU64::U64(v)) => Ok(Some(v)),
        Some(StringOrU64::Str(s)) => s.trim().parse::<u64>().map(Some).map_err(serde::de::Error::custom),
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    #[derive(Deserialize)]
    struct TestRequired {
        #[serde(deserialize_with = "flexible_u64")]
        value: u64,
    }

    #[derive(Deserialize)]
    struct TestOptional {
        #[serde(default, deserialize_with = "flexible_u64_opt")]
        value: Option<u64>,
    }

    #[test]
    fn test_flexible_u64_from_number() {
        let json = r#"{"value": 42}"#;
        let t: TestRequired = serde_json::from_str(json).unwrap();
        assert_eq!(t.value, 42);
    }

    #[test]
    fn test_flexible_u64_from_string() {
        let json = r#"{"value": "42"}"#;
        let t: TestRequired = serde_json::from_str(json).unwrap();
        assert_eq!(t.value, 42);
    }

    #[test]
    fn test_flexible_u64_from_padded_string() {
        let json = r#"{"value": " 42 "}"#;
        let t: TestRequired = serde_json::from_str(json).unwrap();
        assert_eq!(t.value, 42);
    }

    #[test]
    fn test_flexible_u64_invalid_string() {
        let json = r#"{"value": "abc"}"#;
        let result: Result<TestRequired, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_flexible_u64_opt_from_number() {
        let json = r#"{"value": 42}"#;
        let t: TestOptional = serde_json::from_str(json).unwrap();
        assert_eq!(t.value, Some(42));
    }

    #[test]
    fn test_flexible_u64_opt_from_string() {
        let json = r#"{"value": "42"}"#;
        let t: TestOptional = serde_json::from_str(json).unwrap();
        assert_eq!(t.value, Some(42));
    }

    #[test]
    fn test_flexible_u64_opt_null() {
        let json = r#"{"value": null}"#;
        let t: TestOptional = serde_json::from_str(json).unwrap();
        assert_eq!(t.value, None);
    }

    #[test]
    fn test_flexible_u64_opt_missing() {
        let json = r#"{}"#;
        let t: TestOptional = serde_json::from_str(json).unwrap();
        assert_eq!(t.value, None);
    }
}
