use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

/// Errors returned by this crate.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Etsi014Error {
    /// The KME URL could not be parsed.
    #[error("invalid KME URL: {0}")]
    InvalidUrl(#[from] url::ParseError),

    /// The KME URL does not use `https` and insecure HTTP was not allowed.
    ///
    /// ETSI GS QKD 014 (clause 5.1) requires HTTPS with TLS 1.2 or higher.
    #[error("insecure URL scheme `{0}`: ETSI GS QKD 014 requires https")]
    InsecureScheme(String),

    /// Building the HTTP client or talking to the KME failed
    /// (invalid certificate, TLS handshake, connection, timeout, ...).
    #[error("transport error: {0}")]
    Transport(#[from] reqwest::Error),

    /// The KME answered with a non-success HTTP status.
    ///
    /// `body` is `None` when the KME sent no parseable [`ApiError`],
    /// e.g. on `401 Unauthorized`, which has no body per the spec.
    /// In that case `raw` holds the (truncated) body, if there was one,
    /// such as an HTML error page from a reverse proxy.
    #[error(
        "KME returned {status}{}",
        .body.as_ref().map(|b| format!(": {}", b.message)).unwrap_or_default()
    )]
    Api {
        status: StatusCode,
        body: Option<ApiError>,
        raw: Option<String>,
    },

    /// The KME answered with a success status but the body could not be decoded.
    #[error("invalid response from KME: {0}")]
    Decode(#[from] serde_json::Error),

    /// The SAE ID cannot be used as a URL path segment (empty, "." or "..").
    #[error("invalid SAE ID `{0}`")]
    InvalidSaeId(String),
}

impl Etsi014Error {
    /// Classification of a KME error response, or `None` if the error
    /// did not come from a KME response (transport, decoding, configuration).
    ///
    /// Best effort: the spec-defined cases are recognised by status code and
    /// by the exact messages from ETSI GS QKD 014. KMEs that word their
    /// messages differently fall back to [`ApiErrorKind::BadRequest`] or
    /// [`ApiErrorKind::Other`].
    pub fn kind(&self) -> Option<ApiErrorKind> {
        match self {
            Self::Api { status, body, .. } => Some(ApiErrorKind::classify(
                *status,
                body.as_ref().map(|b| b.message.as_str()),
            )),
            _ => None,
        }
    }

    /// HTTP status returned by the KME, if the error came from a KME response.
    pub fn status(&self) -> Option<StatusCode> {
        match self {
            Self::Api { status, .. } => Some(*status),
            Self::Transport(err) => err.status(),
            _ => None,
        }
    }

    /// Error body sent by the KME, if any.
    pub fn api_error(&self) -> Option<&ApiError> {
        match self {
            Self::Api { body, .. } => body.as_ref(),
            _ => None,
        }
    }
}

/// Kind of error reported by a KME, as defined by ETSI GS QKD 014
/// (Tables 4, 6 and 8, clauses 6.2 and 6.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ApiErrorKind {
    /// `401`: the SAE is not authorized. Either TLS client authentication
    /// failed, or (on `dec_keys`) the keys were requested for another slave SAE.
    Unauthorized,
    /// `400` "one or more keys specified are not found on KME":
    /// unknown key IDs, or keys already retrieved.
    KeysNotFound,
    /// `400` "size shall be a multiple of 8".
    SizeNotMultipleOf8,
    /// `400` "not all extension_mandatory parameters are supported".
    ExtensionMandatoryUnsupported,
    /// `400` "not all extension_mandatory request options could be met".
    ExtensionMandatoryUnmet,
    /// `400` "not all extension_optional request options handled".
    ExtensionOptionalNotHandled,
    /// Any other `400`: bad request format.
    BadRequest,
    /// `503`: error on the KME side.
    ServiceUnavailable,
    /// Any other status (e.g. `404`, `500`, `502` from a proxy).
    Other,
}

impl ApiErrorKind {
    fn classify(status: StatusCode, message: Option<&str>) -> Self {
        match status {
            StatusCode::UNAUTHORIZED => Self::Unauthorized,
            StatusCode::SERVICE_UNAVAILABLE => Self::ServiceUnavailable,
            StatusCode::BAD_REQUEST => {
                let message = message
                    .map(|m| m.trim().trim_end_matches('.').to_ascii_lowercase())
                    .unwrap_or_default();
                match message.as_str() {
                    "one or more keys specified are not found on kme" => Self::KeysNotFound,
                    "size shall be a multiple of 8" => Self::SizeNotMultipleOf8,
                    "not all extension_mandatory parameters are supported" => {
                        Self::ExtensionMandatoryUnsupported
                    }
                    "not all extension_mandatory request options could be met" => {
                        Self::ExtensionMandatoryUnmet
                    }
                    "not all extension_optional request options handled" => {
                        Self::ExtensionOptionalNotHandled
                    }
                    _ => Self::BadRequest,
                }
            }
            _ => Self::Other,
        }
    }
}

/// Error data format returned by the KME (ETSI GS QKD 014, clause 6.5).
///
/// Contains a human-readable message and optional structured details.
#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug)]
pub struct ApiError {
    /// Human-readable error message.
    pub message: String,
    /// Optional structured details (arbitrary JSON values).
    pub details: Option<Vec<serde_json::Value>>,
}

impl ApiError {
    /// Create a new [`ApiError`] with only a message.
    pub fn new(msg: String) -> Self {
        Self {
            message: msg,
            details: None,
        }
    }
}
