use serde::{Deserialize, Serialize};

use crate::key::QKDKey;

/// Represents the response from the ETSI 014 `/status` endpoint.
///
/// Provides details about the Key Management Entity (KME) and its
/// supported capabilities, such as key size, limits, and identifiers.
#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug, Default, PartialEq)]
pub struct StatusResponse {
    /// Identifier of the source KME.
    pub source_KME_ID: String,
    /// Identifier of the target KME.
    pub target_KME_ID: String,
    /// Identifier of the master Secure Application Entity (SAE).
    pub master_SAE_ID: String,
    /// Identifier of the slave SAE.
    pub slave_SAE_ID: String,
    /// Current key size in bits.
    pub key_size: u32,
    /// Number of keys currently stored in the KME.
    pub stored_key_count: u32,
    /// Maximum number of keys that can be stored.
    pub max_key_count: u32,
    /// Maximum number of keys that can be retrieved in a single request.
    pub max_key_per_request: u32,
    /// Maximum supported key size in bits.
    pub max_key_size: u32,
    /// Minimum supported key size in bits.
    pub min_key_size: u32,
    /// Maximum number of SAE IDs that can be supported.
    pub max_SAE_ID_count: u32,
}

/// Request body for the `/enc_keys` endpoint to obtain fresh keys.
///
/// - `number`: Number of keys to request.
/// - `size`: Desired key size in bits.
/// - `additional_slave_SAE_IDs`: Optional list of extra slave IDs.
#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Default)]
pub struct KeyRequest {
    pub number: Option<u32>,
    pub size: Option<u32>,
    pub additional_slave_SAE_IDs: Option<Vec<String>>,
}

/// Response body containing a set of QKD keys.
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct KeyResponse {
    /// List of keys returned from the KME.
    pub keys: Vec<QKDKey>,
}

/// Request body for `/dec_keys` to fetch keys by ID.
#[allow(non_snake_case)]
#[derive(Serialize, Deserialize)]
pub struct KeyIdRequest {
    pub key_IDs: Vec<KeyId>,
}

/// Represents a single key identifier.
#[allow(non_snake_case)]
#[derive(Serialize, Deserialize)]
pub struct KeyId {
    pub key_ID: String,
}

impl KeyRequest {
    /// Construct a new [`KeyRequest`] with the given parameters.
    pub fn new(number: Option<u32>, size: Option<u32>) -> Self {
        Self {
            number,
            size,
            additional_slave_SAE_IDs: None,
        }
    }
}

impl KeyIdRequest {
    /// Construct a new [`KeyIdRequest`] with a list of key IDs.
    pub fn new(key_ids: Vec<KeyId>) -> Self {
        Self { key_IDs: key_ids }
    }
}
