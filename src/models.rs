use serde::{Deserialize, Serialize};

use crate::key::QKDKey;

/// Status data format (ETSI GS QKD 014, clause 6.1), returned by
/// [`Endpoint::status`](crate::Endpoint::status).
///
/// Describes the link between the local KME and the KME of the peer SAE,
/// and the limits to respect in a [`KeyRequest`].
///
/// ```
/// use etsi014::StatusResponse;
///
/// let status: StatusResponse = serde_json::from_str(r#"{
///     "source_KME_ID": "KME_A", "target_KME_ID": "KME_B",
///     "master_SAE_ID": "SAE_A", "slave_SAE_ID": "SAE_B",
///     "key_size": 256, "stored_key_count": 25000, "max_key_count": 100000,
///     "max_key_per_request": 128, "max_key_size": 1024, "min_key_size": 64,
///     "max_SAE_ID_count": 0
/// }"#)?;
/// assert_eq!(status.slave_SAE_ID, "SAE_B");
/// assert_eq!(status.key_size, 256);
/// # Ok::<(), serde_json::Error>(())
/// ```
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
    /// Default key size in bits, used when [`KeyRequest::size`] is `None`.
    pub key_size: u32,
    /// Number of keys currently stored in the KME for this SAE pair.
    pub stored_key_count: u32,
    /// Maximum number of keys that can be stored.
    pub max_key_count: u32,
    /// Maximum value of [`KeyRequest::number`].
    pub max_key_per_request: u32,
    /// Maximum supported key size in bits.
    pub max_key_size: u32,
    /// Minimum supported key size in bits.
    pub min_key_size: u32,
    /// Maximum number of [`KeyRequest::additional_slave_SAE_IDs`]; `0` means
    /// the KME does not support multicast keys.
    pub max_SAE_ID_count: u32,
}

/// Key request data format (ETSI GS QKD 014, clause 6.2), sent by
/// [`Endpoint::get_key`](crate::Endpoint::get_key).
///
/// Fields left as `None` let the KME use its defaults.
///
/// ```
/// use etsi014::KeyRequest;
///
/// // Three 256-bit keys
/// let request = KeyRequest::new(Some(3), Some(256));
///
/// // One key of the KME's default size (`StatusResponse::key_size`)
/// let request = KeyRequest::default();
///
/// // Two keys shared with SAE_B (the `slave_sae_id` argument) and SAE_C
/// let request = KeyRequest {
///     additional_slave_SAE_IDs: Some(vec!["SAE_C".into()]),
///     ..KeyRequest::new(Some(2), None)
/// };
/// ```
#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Default)]
pub struct KeyRequest {
    /// Number of keys requested. Defaults to 1, and must not exceed
    /// [`StatusResponse::max_key_per_request`].
    pub number: Option<u32>,
    /// Size of each key in bits. Must be a multiple of 8, within
    /// [`StatusResponse::min_key_size`]`..=`[`StatusResponse::max_key_size`].
    /// Defaults to [`StatusResponse::key_size`].
    pub size: Option<u32>,
    /// Further slave SAEs, besides the one in the URL, that will receive the
    /// same keys (multicast). Limited by [`StatusResponse::max_SAE_ID_count`].
    pub additional_slave_SAE_IDs: Option<Vec<String>>,
}

/// Key container data format (ETSI GS QKD 014, clause 6.3), returned by
/// [`Endpoint::get_key`](crate::Endpoint::get_key) and
/// [`Endpoint::get_key_with_id`](crate::Endpoint::get_key_with_id).
///
/// ```
/// use etsi014::KeyResponse;
///
/// let response: KeyResponse = serde_json::from_str(r#"{"keys": [{
///     "key_ID": "bc490419-7d60-487f-adc1-4ddcc177c139",
///     "key": "wHHVxRwDJs3/bXd38GHP3oe4svTuRpZS0yCC7x4Ly+s="
/// }]}"#)?;
/// assert_eq!(response.keys.len(), 1);
/// # Ok::<(), serde_json::Error>(())
/// ```
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct KeyResponse {
    /// List of keys returned from the KME.
    pub keys: Vec<QKDKey>,
}

/// Key IDs data format (ETSI GS QKD 014, clause 6.4), sent by
/// [`Endpoint::get_key_with_id`](crate::Endpoint::get_key_with_id).
///
/// ```
/// use etsi014::{KeyId, KeyIdRequest};
///
/// let request = KeyIdRequest::new(vec![KeyId {
///     key_ID: "bc490419-7d60-487f-adc1-4ddcc177c139".into(),
/// }]);
/// assert_eq!(
///     serde_json::to_string(&request)?,
///     r#"{"key_IDs":[{"key_ID":"bc490419-7d60-487f-adc1-4ddcc177c139"}]}"#,
/// );
/// # Ok::<(), serde_json::Error>(())
/// ```
#[allow(non_snake_case)]
#[derive(Serialize, Deserialize)]
pub struct KeyIdRequest {
    /// IDs of the keys to retrieve, as received from the master SAE.
    pub key_IDs: Vec<KeyId>,
}

/// A key ID, as found in [`QKDKey::key_ID`](crate::QKDKey::key_ID).
#[allow(non_snake_case)]
#[derive(Serialize, Deserialize)]
pub struct KeyId {
    /// The key ID (usually a UUID).
    pub key_ID: String,
}

impl KeyRequest {
    /// Creates a request for `number` keys of `size` bits each, with no
    /// additional slave SAEs. `None` leaves the choice to the KME.
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
