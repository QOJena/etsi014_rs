use std::path::PathBuf;

use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

/// Errors returned by this crate.
///
/// Errors fall into three groups:
///
/// - Configuration, from [`EndpointBuilder::build`](crate::EndpointBuilder::build):
///   [`InvalidUrl`](Self::InvalidUrl), [`InsecureScheme`](Self::InsecureScheme),
///   [`Io`](Self::Io), [`InvalidPem`](Self::InvalidPem).
/// - Errors reported by the KME: [`Api`](Self::Api). Use [`kind`](Self::kind)
///   to match them against the cases of the spec.
/// - Other request failures: [`Transport`](Self::Transport),
///   [`Decode`](Self::Decode), [`InvalidSaeId`](Self::InvalidSaeId).
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
        /// HTTP status of the response.
        status: StatusCode,
        /// Error body (clause 6.5), if the KME sent one.
        body: Option<ApiError>,
        /// Start of the body (at most 512 bytes) if it was not a valid
        /// [`ApiError`].
        raw: Option<String>,
    },

    /// The KME answered with a success status but the body could not be decoded.
    #[error("invalid response from KME: {0}")]
    Decode(#[from] serde_json::Error),

    /// The SAE ID cannot be used as a URL path segment (empty, "." or "..").
    #[error("invalid SAE ID `{0}`")]
    InvalidSaeId(String),

    /// A certificate or private key file could not be read.
    #[error("failed to read `{}`: {source}", path.display())]
    Io {
        /// File that could not be read.
        path: PathBuf,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// The root CA or the client identity is not valid PEM, or is missing
    /// the certificate or the private key.
    #[error("invalid {what} PEM: {reason}")]
    InvalidPem {
        /// `"root CA"` or `"client identity"`.
        what: &'static str,
        /// Description of the problem.
        reason: String,
    },
}

impl Etsi014Error {
    /// reqwest reports PEM errors as a generic "builder error" with the
    /// details in its source chain, so the whole chain is kept as the reason.
    pub(crate) fn invalid_pem(what: &'static str, err: reqwest::Error) -> Self {
        let mut reason = err.to_string();
        let mut source = std::error::Error::source(&err);
        while let Some(s) = source {
            reason.push_str(&format!(": {s}"));
            source = s.source();
        }
        Self::InvalidPem { what, reason }
    }

    /// Classification of a KME error response, or `None` if the error
    /// did not come from a KME response (transport, decoding, configuration).
    ///
    /// Best effort: the spec-defined cases are recognised by status code and
    /// by the exact messages from ETSI GS QKD 014. KMEs that word their
    /// messages differently fall back to [`ApiErrorKind::BadRequest`] or
    /// [`ApiErrorKind::Other`].
    ///
    /// ```
    /// use etsi014::{ApiError, ApiErrorKind, Etsi014Error};
    /// use reqwest::StatusCode;
    ///
    /// let err = Etsi014Error::Api {
    ///     status: StatusCode::BAD_REQUEST,
    ///     body: Some(ApiError::new("size shall be a multiple of 8".into())),
    ///     raw: None,
    /// };
    /// assert_eq!(err.kind(), Some(ApiErrorKind::SizeNotMultipleOf8));
    ///
    /// let err = Etsi014Error::InvalidSaeId(String::new());
    /// assert_eq!(err.kind(), None);
    /// ```
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
    ///
    /// ```
    /// use etsi014::Etsi014Error;
    /// use reqwest::StatusCode;
    ///
    /// let err = Etsi014Error::Api { status: StatusCode::UNAUTHORIZED, body: None, raw: None };
    /// assert_eq!(err.status(), Some(StatusCode::UNAUTHORIZED));
    /// ```
    pub fn status(&self) -> Option<StatusCode> {
        match self {
            Self::Api { status, .. } => Some(*status),
            Self::Transport(err) => err.status(),
            _ => None,
        }
    }

    /// Error body (clause 6.5) sent by the KME, if any.
    ///
    /// ```no_run
    /// # async fn run(endpoint: etsi014::Endpoint) {
    /// if let Err(e) = endpoint.status("SAE_B").await {
    ///     if let Some(body) = e.api_error() {
    ///         eprintln!("KME says: {} ({:?})", body.message, body.details);
    ///     }
    /// }
    /// # }
    /// ```
    pub fn api_error(&self) -> Option<&ApiError> {
        match self {
            Self::Api { body, .. } => body.as_ref(),
            _ => None,
        }
    }
}

/// Kind of error reported by a KME, as defined by ETSI GS QKD 014
/// (Tables 4, 6 and 8, clauses 6.2 and 6.4).
///
/// Returned by [`Etsi014Error::kind`]. New kinds may be added, so `match`
/// statements need a wildcard arm.
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
