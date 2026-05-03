use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct RoundTrip {
    pub field: String,
    pub original: String,
    pub encrypted: String,
    pub decrypted: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_serializes_with_expected_fields() {
        let value = RoundTrip {
            field: "card_number".into(),
            original: "4242424242424242".into(),
            encrypted: "ev:debug:abc".into(),
            decrypted: "4242424242424242".into(),
        };
        let json = serde_json::to_value(&value).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "field": "card_number",
                "original": "4242424242424242",
                "encrypted": "ev:debug:abc",
                "decrypted": "4242424242424242",
            })
        );
    }

    #[test]
    fn round_trip_deserializes_from_json() {
        let json = serde_json::json!({
            "field": "ssn",
            "original": "123-45-6789",
            "encrypted": "ev:debug:xyz",
            "decrypted": "123-45-6789",
        });
        let value: RoundTrip = serde_json::from_value(json).unwrap();
        assert_eq!(value.field, "ssn");
        assert_eq!(value.encrypted, "ev:debug:xyz");
    }
}
