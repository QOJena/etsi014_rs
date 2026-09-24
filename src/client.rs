use reqwest::{Client, RequestBuilder};
use serde::de::DeserializeOwned;
use url::Url;

use crate::{
    Etsi014Error, KeyIdRequest, KeyRequest, KeyResponse, StatusResponse, config::EndpointBuilder, core::{self, Method},
};

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
/// let status = endpoint.status("SAE_B").await?;
/// println!("KME status: {:?}", status);
///
/// let request = KeyRequest::new(Some(3), Some(256));
/// let keys = endpoint.get_key("SAE_B", &request).await?;
/// println!("Received {} keys", keys.keys.len());
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct Endpoint {
    /// Base URL of the local KME (`https://{KME_hostname}`).
    pub url: Url,
    /// Internal client used to send requests.
    pub(crate) client: Client,
}

impl Endpoint {

    pub fn builder(url: impl Into<String>) -> EndpointBuilder<Self> {
        EndpointBuilder::new(url)
    }

    /// Build the full URL for a given ETSI 014 path.
    fn build_url(&self, peer_sae_id: &str, method: Method) -> Result<Url, Etsi014Error> {
        core::build_url(
            &self.url,
            peer_sae_id,
            method
        )
    }

    /// Send a request and decode the KME response into `T`.
    async fn send<T: DeserializeOwned>(&self, request: RequestBuilder) -> Result<T, Etsi014Error> {
        let response = request.send().await?;
        let status = response.status();
        if !status.is_success() {
            log::debug!("KME returned {} for {}", status, response.url());
        }
        let body = response.bytes().await?;
        core::decode(status, &body)
    }

    /// Get status (ETSI GS QKD 014, clause 5.2): `GET /api/v1/keys/{slave_SAE_ID}/status`.
    ///
    /// `slave_sae_id`: the peer SAE you would request keys for (this SAE acts as master).
    pub async fn status(&self, slave_sae_id: &str) -> Result<StatusResponse, Etsi014Error> {
        let url = self.build_url(slave_sae_id, Method::Status)?;
        self.send(self.client.get(url)).await
    }

    /// Get key (clause 5.3): `POST /api/v1/keys/{slave_SAE_ID}/enc_keys`.
    ///
    /// `slave_sae_id`: the peer SAE that will later fetch the same keys via `dec_keys`.
    /// This SAE becomes the master for the returned keys.
    pub async fn get_key(
        &self,
        slave_sae_id: &str,
        key_request: &KeyRequest,
    ) -> Result<KeyResponse, Etsi014Error> {
        let url = self.build_url(slave_sae_id, Method::EncKeys)?;
        self.send(self.client.post(url).json(key_request)).await
    }

    /// Get key with key IDs (clause 5.4): `POST /api/v1/keys/{master_SAE_ID}/dec_keys`.
    ///
    /// `master_sae_id`: the peer SAE that originally obtained these keys via `enc_keys`.
    /// This SAE acts as slave for them.
    ///
    /// The KME removes the keys from its pool once delivered, so if the response
    /// is lost (e.g. a timeout while reading it) the keys cannot be fetched again.
    pub async fn get_key_with_id(
        &self,
        master_sae_id: &str,
        key_with_id: &KeyIdRequest,
    ) -> Result<KeyResponse, Etsi014Error> {
        let url = self.build_url(master_sae_id, Method::DecKeys)?;
        self.send(self.client.post(url).json(key_with_id)).await
    }
}
