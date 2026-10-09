use serde::{Deserialize, Serialize};

/// A key delivered by the KME (ETSI GS QKD 014, clause 6.3), as found in
/// [`KeyResponse::keys`](crate::KeyResponse::keys).
///
/// The key material is kept as sent by the KME, base64-encoded; decode it
/// with a base64 crate of your choice. Treat it as a secret: avoid logging
/// it, including through this type's `Debug` output.
#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug)]
pub struct QKDKey {
    /// ID of the key (usually a UUID). Share it with the slave SAE so it can
    /// fetch the same key with
    /// [`Endpoint::get_key_with_id`](crate::Endpoint::get_key_with_id).
    pub key_ID: String,
    /// Key material, base64-encoded.
    pub key: String,
}

/// Size in bytes of a UUID used as a key identifier.
const SIZE_UUID: usize = 16;
/// Size in bytes of a QKD key.
const SIZE_KEY: usize = 32;

/// A key in binary form: a 16-byte UUID and 256 bits of key material.
pub struct QKDKeyPair {
    /// Key ID, as the 16 bytes of a UUID.
    pub id: [u8; SIZE_UUID],
    /// Key material (256 bits).
    pub key: [u8; SIZE_KEY],
}
