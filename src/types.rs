use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct RoundTrip {
    pub field: String,
    pub original: String,
    pub encrypted: String,
    pub decrypted: String,
}
