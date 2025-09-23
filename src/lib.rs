
// Conditionally import blocking
#[cfg(feature="blocking")]
pub mod blocking;

mod core;

use reqwest::{Client, Identity};
use serde::{Deserialize, Serialize};

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

/// Represents a single QKD key (ID and material).
#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug)]
pub struct QKDKey {
    /// Unique identifier for the key.
    pub key_ID: String,
    /// The actual key material (usually base64-encoded).
    pub key: String,
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

/// Error structure returned by the ETSI 014 API.
///
/// Contains a human-readable message and optional structured details.
#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug)]
pub struct Error {
    /// Human-readable error message.
    pub message: String,
    /// Optional structured details (arbitrary JSON values).
    pub details: Option<Vec<serde_json::Value>>,
}

/// Defines the ETSI 014 endpoint of a QKD device (a KME).
///
/// This struct encapsulates the configuration and HTTP client
/// used to send requests to the KME. It provides methods to:
///
/// - Query the status of the KME (`/status`)
/// - Request new keys (`/enc_keys`)
/// - Retrieve keys by ID (`/dec_keys`)
///
/// # Example
/// ```ignore
/// use qkd_client::{Endpoint, KeyRequest};
///
/// # async fn demo() -> Result<(), Box<dyn std::error::Error>> {
/// let endpoint = Endpoint::new("https://kme.example.com", "slave1", None, None)?;
/// let status = endpoint.status().await?;
/// println!("KME status: {:?}", status);
///
/// let request = KeyRequest::new(Some(3), Some(256));
/// let keys = endpoint.get_key(request).await?;
/// println!("Received {} keys", keys.keys.len());
/// # Ok(())
/// # }
/// ```
#[allow(non_snake_case)]
#[derive(Debug, Clone)]
pub struct Endpoint {
    /// Hostname or IP of the KME.
    pub KME_hostname: String,
    /// Slave SAE identifier for this client.
    pub slave_SAE_ID: String,
    /// Internal HTTP client used to send requests.
    pub(crate) client: Client,
    /// Whether TLS is enabled for this connection.
    tls: bool,
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

impl Error {
    /// Create a new [`Error`] with only a message.
    pub fn new(msg: String) -> Self {
        Self {
            message: msg,
            details: None,
        }
    }
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


impl Endpoint {

    /// Create a new [`Endpoint`] pointing to a KME.
    ///
    /// # Arguments
    /// - `kme_hostname`: Hostname or IP of the KME.
    /// - `slave_sae_id`: Local slave SAE identifier.
    /// - `cert`: Optional path to a PEM root certificate file.
    /// - `identity`: Optional path to a PEM client certificate + key.
    ///
    /// # Errors
    /// Returns an error if certificates cannot be loaded or if the
    /// HTTP client cannot be built.
    pub fn new(kme_hostname: &str, slave_sae_id: &str, cert: Option<String>, identity: Option<String>) -> Result<Self, anyhow::Error> {

        let mut client_builder = reqwest::Client::builder()
            .user_agent("etsi014-client/0.1.0")
            .use_rustls_tls();

        let tls = cert.is_some() || identity.is_some();

        if let Some(c) = cert {
            let certificate = std::fs::read(c)?;
            let c = reqwest::Certificate::from_pem(&certificate)?;

            client_builder = client_builder.add_root_certificate(c);
        }

        if let Some(id) = identity {

            let identity = std::fs::read(id)?;
            let id = Identity::from_pem(&identity)?;

            client_builder = client_builder.identity(id);
        }

        let client = client_builder.build()?;


        Ok(Self {
            KME_hostname: String::from(kme_hostname),
            slave_SAE_ID: String::from(slave_sae_id),
            client,
            tls
        })
    }

    /// Build the full URL for a given ETSI 014 path.
    fn build_url(&self, path: &str) -> String {
        core::build_url(&self.KME_hostname, self.slave_SAE_ID.as_str(), path, self.tls)
    }

    /// Query the `/status` endpoint.
    ///
    /// Returns information about the KME and its current capabilities.
    pub async fn status(&self) -> Result<StatusResponse, Error> {
        let resp = self.client.get(self.build_url("status")).send().await;
        
        let response = match resp {
            Ok(resp) => {
                if resp.status().is_client_error() || resp.status().is_server_error() {
                    log::error!("Error response: {:?}", resp);
                    match resp.json::<Error>().await {
                        Ok(error) => return Err(error),
                        Err(err) => return Err(Error { message: err.to_string(), details: None })
                    }
                }
                resp
            },
            Err(err) => {
            log::error!("Error sending request: {}", err);
                return Err(Error { message: err.to_string(), details: None });
            }
            
        };

        log::debug!("Response: {:?}", response);
        
        match response.json::<StatusResponse>().await {
            Ok(status) => Ok(status),
            Err(err) => Err(Error {
                message: err.to_string(),
                details: None
            })
        }
    }

    /// Request fresh keys from the `/enc_keys` endpoint.
    ///
    /// # Arguments
    /// - `key_request`: Parameters for the key request.
    ///
    /// # Returns
    /// A [`KeyResponse`] containing the new keys.
    pub async fn get_key(&self, key_request: KeyRequest) -> Result<KeyResponse, Error> {

        let resp = self.client.post(self.build_url("enc_keys"))
                    .json(&key_request).send().await;
        

        match resp {
            Ok(response) => {
                if response.status().is_client_error() || response.status().is_server_error() {
                    log::error!("Error response: {:?}", response);
                    match response.json::<Error>().await {
                        Ok(error) => Err(error),
                        Err(err) => Err(Error { message: err.to_string(), details: None })
                    }
                } else {
                    log::error!("Error response: {:?}", response);
                    match response.json::<KeyResponse>().await {
                        Ok(key) => Ok(key),
                        Err(err) => Err(Error { message: err.to_string(), details: None })
                    }
                }
                
            },
            Err(err) => {
                log::error!("Error response: {:?}", err);
                Err(Error { message: err.to_string(), details: None })
            }
        }
    }


    /// Retrieve specific keys by their IDs via the `/dec_keys` endpoint.
    ///
    /// # Arguments
    /// - `key_with_id`: List of key IDs to fetch.
    ///
    /// # Returns
    /// A [`KeyResponse`] containing the requested keys.
    pub async fn get_key_with_id(&self, key_with_id: &KeyIdRequest) -> Result<KeyResponse, Error> {
        let client = reqwest::Client::new();

        let resp = client.post(self.build_url("dec_keys"))
                    .json(&key_with_id).send().await;

        match resp {
            Ok(response) => {
                if response.status().is_client_error() || response.status().is_server_error() {
                    match response.json::<Error>().await {
                        Ok(error) => Err(error),
                        Err(err) => Err(Error { message: err.to_string(), details: None })
                    }
                } else {
                    match response.json::<KeyResponse>().await {
                        Ok(key) => Ok(key),
                        Err(err) => Err(Error { message: err.to_string(), details: None })
                    }
                }
                
            },
            Err(err) => {
                Err(Error { message: err.to_string(), details: None })
            }
        }
    }


}


