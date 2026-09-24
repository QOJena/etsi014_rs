use reqwest::blocking::{Client, RequestBuilder};
use serde::de::DeserializeOwned;
use url::Url;

use crate::{
    core::{self, Method},
    Etsi014Error, KeyIdRequest, KeyRequest, KeyResponse, StatusResponse,
};

/// Blocking version of [`crate::Endpoint`].
///
/// Do not create or drop it inside an async runtime (e.g. Tokio):
/// `reqwest`'s blocking client panics there.
#[derive(Debug, Clone)]
pub struct Endpoint {
    /// Base URL of the local KME (`https://{KME_hostname}`).
    pub url: Url,
    /// Internal client used to send requests.
    pub(crate) client: Client,
}

impl Endpoint {
    /// Build the full URL for a given ETSI 014 path.
    fn build_url(&self, peer_sae_id: &str, method: Method) -> Result<Url, Etsi014Error> {
        core::build_url(&self.url, peer_sae_id, method)
    }

    /// Send a request and decode the KME response into `T`.
    fn send<T: DeserializeOwned>(&self, request: RequestBuilder) -> Result<T, Etsi014Error> {
        let response = request.send()?;
        let status = response.status();
        if !status.is_success() {
            log::debug!("KME returned {} for {}", status, response.url());
        }
        let body = response.bytes()?;
        core::decode(status, &body)
    }

    /// Get status (ETSI GS QKD 014, clause 5.2): `GET /api/v1/keys/{slave_SAE_ID}/status`.
    ///
    /// `slave_sae_id`: the peer SAE you would request keys for (this SAE acts as master).
    pub fn status(&self, slave_sae_id: &str) -> Result<StatusResponse, Etsi014Error> {
        let url = self.build_url(slave_sae_id, Method::Status)?;
        self.send(self.client.get(url))
    }

    /// Get key (clause 5.3): `POST /api/v1/keys/{slave_SAE_ID}/enc_keys`.
    ///
    /// `slave_sae_id`: the peer SAE that will later fetch the same keys via `dec_keys`.
    /// This SAE becomes the master for the returned keys.
    pub fn get_key(
        &self,
        slave_sae_id: &str,
        key_request: &KeyRequest,
    ) -> Result<KeyResponse, Etsi014Error> {
        let url = self.build_url(slave_sae_id, Method::EncKeys)?;
        self.send(self.client.post(url).json(key_request))
    }

    /// Get key with key IDs (clause 5.4): `POST /api/v1/keys/{master_SAE_ID}/dec_keys`.
    ///
    /// `master_sae_id`: the peer SAE that originally obtained these keys via `enc_keys`.
    /// This SAE acts as slave for them.
    ///
    /// The KME removes the keys from its pool once delivered, so if the response
    /// is lost (e.g. a timeout while reading it) the keys cannot be fetched again.
    pub fn get_key_with_id(
        &self,
        master_sae_id: &str,
        key_with_id: &KeyIdRequest,
    ) -> Result<KeyResponse, Etsi014Error> {
        let url = self.build_url(master_sae_id, Method::DecKeys)?;
        self.send(self.client.post(url).json(key_with_id))
    }
}

#[cfg(test)]
mod test {
    use crate::{config::EndpointBuilder, Etsi014Error, KeyId, KeyIdRequest, KeyRequest};

    use super::Endpoint;

    /// Endpoint pointing to a port where nothing listens.
    fn unreachable_endpoint() -> Endpoint {
        EndpointBuilder::<Endpoint>::new("http://127.0.0.1:8888")
            .danger_allow_insecure_http()
            .build()
            .unwrap()
    }

    #[test]
    fn status_test() {
        let result = unreachable_endpoint().status("bob");
        assert!(matches!(result, Err(Etsi014Error::Transport(_))));
    }

    #[test]
    fn get_key_test() {
        let request = KeyRequest {
            number: Some(3),
            size: Some(256),
            additional_slave_SAE_IDs: None,
        };

        let result = unreachable_endpoint().get_key("bob", &request);
        assert!(matches!(result, Err(Etsi014Error::Transport(_))));
    }

    #[test]
    fn get_key_with_id_test() {
        let request = KeyIdRequest {
            key_IDs: vec![KeyId {
                key_ID: "0".to_string(),
            }],
        };

        let result = unreachable_endpoint().get_key_with_id("alice", &request);
        assert!(matches!(result, Err(Etsi014Error::Transport(_))));
    }
}
