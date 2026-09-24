use serde::{Deserialize, Serialize};

/// Represents a single QKD key (ID and material).
#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug)]
pub struct QKDKey {
    /// Unique identifier for the key.
    pub key_ID: String,
    /// The actual key material (usually base64-encoded).
    pub key: String,
}

/// Size in bytes of a UUID used as a key identifier.
const SIZE_UUID: usize = 16;
/// Size in bytes of a QKD key.
const SIZE_KEY: usize = 32;

/// Represents a QKD key pair in binary format (UUID + key material).
pub struct QKDKeyPair {
    pub id: [u8; SIZE_UUID],
    pub key: [u8; SIZE_KEY],
}
