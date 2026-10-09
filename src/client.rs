use reqwest::{Client, RequestBuilder};
use serde::de::DeserializeOwned;
use url::Url;

use crate::{
    Etsi014Error, KeyIdRequest, KeyRequest, KeyResponse, StatusResponse,
    config::EndpointBuilder,
    core::{self, Method},
};

/// Async client for the ETSI GS QKD 014 API of a KME.
///
/// An `Endpoint` holds the base URL of the local KME and a configured HTTP
/// client. Create one with [`Endpoint::builder`], then call:
///
/// - [`status`](Self::status): status of the link to a peer SAE (`/status`)
/// - [`get_key`](Self::get_key): new keys, as master SAE (`/enc_keys`)
/// - [`get_key_with_id`](Self::get_key_with_id): keys by ID, as slave SAE (`/dec_keys`)
///
/// Cloning an `Endpoint` is cheap and shares the underlying connection pool,
/// so create it once and clone it where needed instead of rebuilding it.
///
/// For a synchronous client, see `blocking::Endpoint` (`blocking` feature).
///
/// # Example
///
/// ```no_run
/// use etsi014::{Endpoint, KeyRequest};
///
/// # #[tokio::main]
/// # async fn main() -> Result<(), etsi014::Etsi014Error> {
/// let endpoint = Endpoint::builder("https://kme-a.example.com")
///     .root_ca_file("certs/ca.crt")
///     .identity_files("certs/sae_a.crt", "certs/sae_a.key")
///     .build()?;
///
/// let status = endpoint.status("SAE_B").await?;
/// println!("KME status: {status:?}");
///
/// let response = endpoint.get_key("SAE_B", &KeyRequest::new(Some(3), Some(256))).await?;
/// println!("Received {} keys", response.keys.len());
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct Endpoint {
    /// Base URL of the local KME (`https://{KME_hostname}`).
    ///
    /// Request paths (`/api/v1/keys/...`) are appended to it, so it may
    /// include a path prefix, e.g. `https://kme.example.com/qkd`.
    pub url: Url,
    /// Internal client used to send requests.
    pub(crate) client: Client,
}

impl Endpoint {
    /// Starts building an endpoint for the KME at `url`
    /// (`https://{KME_hostname}`).
    ///
    /// See [`EndpointBuilder`] for the TLS and timeout options. The URL is
    /// only validated by [`EndpointBuilder::build`].
    ///
    /// ```
    /// use etsi014::Endpoint;
    ///
    /// let endpoint = Endpoint::builder("https://kme.example.com").build()?;
    /// # Ok::<(), etsi014::Etsi014Error>(())
    /// ```
    pub fn builder(url: impl Into<String>) -> EndpointBuilder<Self> {
        EndpointBuilder::new(url)
    }

    /// Build the full URL for a given ETSI 014 path.
    fn build_url(&self, peer_sae_id: &str, method: Method) -> Result<Url, Etsi014Error> {
        core::build_url(&self.url, peer_sae_id, method)
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
    ///
    /// Use it to check that the KME knows the peer SAE and to learn its
    /// limits (key sizes, keys per request) before calling
    /// [`get_key`](Self::get_key).
    ///
    /// # Errors
    ///
    /// - [`Etsi014Error::InvalidSaeId`] if `slave_sae_id` is empty, `.` or `..`
    ///   (no request is sent).
    /// - [`Etsi014Error::Api`] if the KME answers with an error; see
    ///   [`Etsi014Error::kind`].
    /// - [`Etsi014Error::Transport`] on TLS, connection or timeout errors.
    /// - [`Etsi014Error::Decode`] if the KME answers with an unexpected body.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # async fn run(endpoint: etsi014::Endpoint) -> Result<(), etsi014::Etsi014Error> {
    /// let status = endpoint.status("SAE_B").await?;
    /// println!(
    ///     "{} keys stored, up to {} per request, {}..={} bits",
    ///     status.stored_key_count,
    ///     status.max_key_per_request,
    ///     status.min_key_size,
    ///     status.max_key_size,
    /// );
    /// # Ok(())
    /// # }
    /// ```
    pub async fn status(&self, slave_sae_id: &str) -> Result<StatusResponse, Etsi014Error> {
        let url = self.build_url(slave_sae_id, Method::Status)?;
        self.send(self.client.get(url)).await
    }

    /// Get key (clause 5.3): `POST /api/v1/keys/{slave_SAE_ID}/enc_keys`.
    ///
    /// `slave_sae_id`: the peer SAE that will later fetch the same keys via `dec_keys`.
    /// This SAE becomes the master for the returned keys.
    ///
    /// Then share the returned key IDs (never the keys) with the slave SAE,
    /// which fetches the same keys from its own KME with
    /// [`get_key_with_id`](Self::get_key_with_id).
    ///
    /// # Errors
    ///
    /// - [`Etsi014Error::InvalidSaeId`] if `slave_sae_id` is empty, `.` or `..`
    ///   (no request is sent).
    /// - [`Etsi014Error::Api`] if the KME answers with an error; see
    ///   [`Etsi014Error::kind`].
    ///   For example [`ApiErrorKind::SizeNotMultipleOf8`](crate::ApiErrorKind::SizeNotMultipleOf8)
    ///   or [`ApiErrorKind::BadRequest`](crate::ApiErrorKind::BadRequest) if
    ///   `key_request` exceeds the limits reported by [`status`](Self::status).
    /// - [`Etsi014Error::Transport`] on TLS, connection or timeout errors.
    /// - [`Etsi014Error::Decode`] if the KME answers with an unexpected body.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use etsi014::{KeyId, KeyRequest};
    ///
    /// # async fn run(endpoint: etsi014::Endpoint) -> Result<(), etsi014::Etsi014Error> {
    /// // Two 256-bit keys shared with SAE_B
    /// let response = endpoint.get_key("SAE_B", &KeyRequest::new(Some(2), Some(256))).await?;
    ///
    /// for key in &response.keys {
    ///     println!("{} -> {} (base64)", key.key_ID, key.key);
    /// }
    ///
    /// // IDs to send to SAE_B
    /// let ids: Vec<KeyId> = response
    ///     .keys
    ///     .iter()
    ///     .map(|k| KeyId { key_ID: k.key_ID.clone() })
    ///     .collect();
    /// # Ok(())
    /// # }
    /// ```
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
    ///
    /// # Errors
    ///
    /// - [`Etsi014Error::InvalidSaeId`] if `master_sae_id` is empty, `.` or `..`
    ///   (no request is sent).
    /// - [`Etsi014Error::Api`] if the KME answers with an error; see
    ///   [`Etsi014Error::kind`].
    ///   In particular [`ApiErrorKind::KeysNotFound`](crate::ApiErrorKind::KeysNotFound)
    ///   if a key ID is unknown or was already retrieved, and
    ///   [`ApiErrorKind::Unauthorized`](crate::ApiErrorKind::Unauthorized) if
    ///   the keys were requested for another slave SAE.
    /// - [`Etsi014Error::Transport`] on TLS, connection or timeout errors.
    /// - [`Etsi014Error::Decode`] if the KME answers with an unexpected body.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use etsi014::{KeyId, KeyIdRequest};
    ///
    /// # async fn run(endpoint: etsi014::Endpoint) -> Result<(), etsi014::Etsi014Error> {
    /// // Key ID received from the master SAE, SAE_A
    /// let request = KeyIdRequest::new(vec![KeyId {
    ///     key_ID: "bc490419-7d60-487f-adc1-4ddcc177c139".into(),
    /// }]);
    /// let response = endpoint.get_key_with_id("SAE_A", &request).await?;
    /// assert_eq!(response.keys[0].key_ID, "bc490419-7d60-487f-adc1-4ddcc177c139");
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_key_with_id(
        &self,
        master_sae_id: &str,
        key_with_id: &KeyIdRequest,
    ) -> Result<KeyResponse, Etsi014Error> {
        let url = self.build_url(master_sae_id, Method::DecKeys)?;
        self.send(self.client.post(url).json(key_with_id)).await
    }
}
