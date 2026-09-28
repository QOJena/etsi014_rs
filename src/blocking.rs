use reqwest::blocking::{Client, RequestBuilder};
use serde::de::DeserializeOwned;
use url::Url;

use crate::{
    Etsi014Error, KeyIdRequest, KeyRequest, KeyResponse, StatusResponse,
    config::EndpointBuilder,
    core::{self, Method},
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
    pub fn builder(url: impl Into<String>) -> EndpointBuilder<Self> {
        EndpointBuilder::new(url)
    }

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
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::TcpListener;
    use std::thread::{self, JoinHandle};

    use reqwest::StatusCode;

    use super::Endpoint;
    use crate::{ApiErrorKind, Etsi014Error, KeyId, KeyIdRequest, KeyRequest};

    const STATUS_JSON: &str = r#"{
        "source_KME_ID": "KME_A", "target_KME_ID": "KME_B",
        "master_SAE_ID": "SAE_A", "slave_SAE_ID": "SAE_B",
        "key_size": 256, "stored_key_count": 25000, "max_key_count": 100000,
        "max_key_per_request": 128, "max_key_size": 1024, "min_key_size": 64,
        "max_SAE_ID_count": 0
    }"#;

    const KEYS_JSON: &str = r#"{"keys": [
        {"key_ID": "bc490419-7d60-487f-adc1-4ddcc177c139", "key": "wHHVxRwDJs3/bXd38GHP3oe4svTuRpZS0yCC7x4Ly+s="}
    ]}"#;

    /// Request as seen by the fake KME.
    struct Received {
        request_line: String,
        body: String,
    }

    /// One-shot fake KME: answers the first request with `status` and `body`,
    /// and hands back what it received.
    fn serve_once(status: &'static str, body: &'static str) -> (String, JoinHandle<Received>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());

        let handle = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream);

            let mut request_line = String::new();
            reader.read_line(&mut request_line).unwrap();

            let mut content_length = 0;
            loop {
                let mut header = String::new();
                reader.read_line(&mut header).unwrap();
                if header == "\r\n" {
                    break;
                }
                if let Some((name, value)) = header.split_once(':')
                    && name.eq_ignore_ascii_case("content-length")
                {
                    content_length = value.trim().parse().unwrap();
                }
            }
            let mut request_body = vec![0; content_length];
            reader.read_exact(&mut request_body).unwrap();

            let response = format!(
                "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            reader.get_mut().write_all(response.as_bytes()).unwrap();

            Received {
                request_line: request_line.trim_end().to_string(),
                body: String::from_utf8(request_body).unwrap(),
            }
        });

        (url, handle)
    }

    fn endpoint(url: &str) -> Endpoint {
        Endpoint::builder(url)
            .danger_allow_insecure_http()
            .build()
            .unwrap()
    }

    // --- builder ---

    #[test]
    fn builder_accepts_https() {
        let endpoint = Endpoint::builder("https://kme.example.com:443")
            .build()
            .unwrap();
        assert_eq!(endpoint.url.as_str(), "https://kme.example.com/");
    }

    #[test]
    fn builder_rejects_http_by_default() {
        let result = Endpoint::builder("http://kme.example.com").build();
        assert!(matches!(result, Err(Etsi014Error::InsecureScheme(scheme)) if scheme == "http"));
    }

    #[test]
    fn builder_accepts_http_when_allowed() {
        let result = Endpoint::builder("http://kme.example.com")
            .danger_allow_insecure_http()
            .build();
        assert!(result.is_ok());
    }

    #[test]
    fn builder_rejects_invalid_url() {
        let result = Endpoint::builder("not a url").build();
        assert!(matches!(result, Err(Etsi014Error::InvalidUrl(_))));
    }

    // --- requests ---

    #[test]
    fn status_uses_slave_sae_id() {
        let (url, server) = serve_once("200 OK", STATUS_JSON);

        let status = endpoint(&url).status("SAE_B").unwrap();

        assert_eq!(
            server.join().unwrap().request_line,
            "GET /api/v1/keys/SAE_B/status HTTP/1.1"
        );
        assert_eq!(status.slave_SAE_ID, "SAE_B");
        assert_eq!(status.max_key_per_request, 128);
    }

    #[test]
    fn get_key_posts_request_to_enc_keys() {
        let (url, server) = serve_once("200 OK", KEYS_JSON);

        let keys = endpoint(&url)
            .get_key("SAE_B", &KeyRequest::new(Some(1), Some(256)))
            .unwrap();

        let received = server.join().unwrap();
        assert_eq!(
            received.request_line,
            "POST /api/v1/keys/SAE_B/enc_keys HTTP/1.1"
        );
        let body: serde_json::Value = serde_json::from_str(&received.body).unwrap();
        assert_eq!(body["number"], 1);
        assert_eq!(body["size"], 256);
        assert_eq!(keys.keys.len(), 1);
        assert_eq!(keys.keys[0].key_ID, "bc490419-7d60-487f-adc1-4ddcc177c139");
    }

    #[test]
    fn get_key_with_id_posts_key_ids_to_dec_keys() {
        let (url, server) = serve_once("200 OK", KEYS_JSON);

        let request = KeyIdRequest::new(vec![KeyId {
            key_ID: "bc490419-7d60-487f-adc1-4ddcc177c139".to_string(),
        }]);
        let keys = endpoint(&url).get_key_with_id("SAE_A", &request).unwrap();

        let received = server.join().unwrap();
        assert_eq!(
            received.request_line,
            "POST /api/v1/keys/SAE_A/dec_keys HTTP/1.1"
        );
        let body: serde_json::Value = serde_json::from_str(&received.body).unwrap();
        assert_eq!(
            body["key_IDs"][0]["key_ID"],
            "bc490419-7d60-487f-adc1-4ddcc177c139"
        );
        assert_eq!(keys.keys.len(), 1);
    }

    #[test]
    fn sae_id_is_url_encoded() {
        let (url, server) = serve_once("200 OK", STATUS_JSON);

        endpoint(&url).status("SAE/B ?").unwrap();

        assert_eq!(
            server.join().unwrap().request_line,
            "GET /api/v1/keys/SAE%2FB%20%3F/status HTTP/1.1"
        );
    }

    #[test]
    fn invalid_sae_id_is_rejected_without_request() {
        // Nothing listens here: an attempted request would be a Transport error
        let result = endpoint("http://127.0.0.1:1").status("..");
        assert!(matches!(result, Err(Etsi014Error::InvalidSaeId(id)) if id == ".."));
    }

    // --- error responses ---

    #[test]
    fn unauthorized_without_body() {
        let (url, server) = serve_once("401 Unauthorized", "");

        let err = endpoint(&url).status("SAE_B").unwrap_err();
        server.join().unwrap();

        assert!(matches!(err, Etsi014Error::Api { body: None, .. }));
        assert_eq!(err.status(), Some(StatusCode::UNAUTHORIZED));
        assert_eq!(err.kind(), Some(ApiErrorKind::Unauthorized));
    }

    #[test]
    fn keys_not_found() {
        let (url, server) = serve_once(
            "400 Bad Request",
            r#"{"message": "one or more keys specified are not found on KME"}"#,
        );

        let request = KeyIdRequest::new(vec![KeyId {
            key_ID: "bc490419-7d60-487f-adc1-4ddcc177c139".to_string(),
        }]);
        let err = endpoint(&url)
            .get_key_with_id("SAE_A", &request)
            .unwrap_err();
        server.join().unwrap();

        assert_eq!(err.kind(), Some(ApiErrorKind::KeysNotFound));
        assert_eq!(
            err.api_error().unwrap().message,
            "one or more keys specified are not found on KME"
        );
    }

    #[test]
    fn invalid_success_body_is_decode_error() {
        let (url, server) = serve_once("200 OK", r#"{"unexpected": true}"#);

        let result = endpoint(&url).get_key("SAE_B", &KeyRequest::default());
        server.join().unwrap();

        assert!(matches!(result, Err(Etsi014Error::Decode(_))));
    }

    #[test]
    fn connection_refused_is_transport_error() {
        // Bind and release a port so nothing listens on it
        let addr = TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap();

        let result = endpoint(&format!("http://{addr}")).status("SAE_B");

        assert!(matches!(result, Err(Etsi014Error::Transport(_))));
    }
}
