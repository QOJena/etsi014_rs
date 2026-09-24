use reqwest::StatusCode;
use serde::de::DeserializeOwned;
use url::Url;

use crate::error::{ApiError, Etsi014Error};

const VERSION: &str = "v1";

/// Maximum number of bytes of a non-JSON error body kept in [`Etsi014Error::Api`].
const MAX_RAW_ERROR_BODY: usize = 512;

#[derive(Clone, Copy)]
pub(crate) enum Method {
    Status,
    EncKeys,
    DecKeys
}

impl Method {
    fn as_str(self) -> &'static str {
        match self {
            Method::Status => "status",
            Method::EncKeys => "enc_keys",
            Method::DecKeys => "dec_keys"
        }
    }
}


pub(crate) fn build_url(base: &Url, sae_id: &str, method: Method) -> Result<Url, Etsi014Error> {
    if sae_id.is_empty() || sae_id == "." || sae_id == ".." {
        return Err(Etsi014Error::InvalidSaeId(sae_id.to_owned()));
    }

    let mut url = base.clone();
    url.path_segments_mut()
        .expect("Validated http(s) URL always has a path")
        .pop_if_empty()
        .extend(["api", VERSION, "keys", sae_id, method.as_str()]);

    Ok(url)
}

/// Turn a KME response (status + body) into the expected type or an [`Etsi014Error`].
///
/// - 2xx: `body` is decoded as `T`.
/// - otherwise: `body` is decoded as [`ApiError`] if possible (spec: 400 and 503);
///   it may also be empty (spec: 401) or not JSON at all (e.g. a proxy error page),
///   in which case a truncated copy is kept in `raw`.
pub(crate) fn decode<T: DeserializeOwned>(status: StatusCode, body: &[u8]) -> Result<T, Etsi014Error> {
    if status.is_success() {
        return Ok(serde_json::from_slice(body)?);
    }

    let api = serde_json::from_slice::<ApiError>(body).ok();
    let raw = if api.is_none() && !body.is_empty() {
        let end = body.len().min(MAX_RAW_ERROR_BODY);
        Some(String::from_utf8_lossy(&body[..end]).into_owned())
    } else {
        None
    };

    Err(Etsi014Error::Api { status, body: api, raw })
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;
    use crate::error::ApiErrorKind;

    #[derive(Debug, Deserialize, PartialEq)]
    struct Dummy {
        value: u32,
    }

    fn err(status: StatusCode, body: &str) -> Etsi014Error {
        decode::<Dummy>(status, body.as_bytes()).unwrap_err()
    }

    #[test]
    fn success_is_decoded() {
        let ok: Dummy = decode(StatusCode::OK, br#"{"value": 7}"#).unwrap();
        assert_eq!(ok, Dummy { value: 7 });
    }

    #[test]
    fn success_with_invalid_body_is_decode_error() {
        assert!(matches!(err(StatusCode::OK, ""), Etsi014Error::Decode(_)));
        assert!(matches!(err(StatusCode::OK, r#"{"other": 1}"#), Etsi014Error::Decode(_)));
    }

    #[test]
    fn unauthorized_without_body() {
        let e = err(StatusCode::UNAUTHORIZED, "");
        assert!(matches!(e, Etsi014Error::Api { body: None, raw: None, .. }));
        assert_eq!(e.status(), Some(StatusCode::UNAUTHORIZED));
        assert_eq!(e.kind(), Some(ApiErrorKind::Unauthorized));
    }

    #[test]
    fn bad_request_with_spec_error_body() {
        let e = err(
            StatusCode::BAD_REQUEST,
            r#"{"message": "one or more keys specified are not found on KME"}"#,
        );
        assert_eq!(e.api_error().unwrap().message, "one or more keys specified are not found on KME");
        assert_eq!(e.kind(), Some(ApiErrorKind::KeysNotFound));
    }

    #[test]
    fn spec_messages_are_classified() {
        let cases = [
            ("size shall be a multiple of 8", ApiErrorKind::SizeNotMultipleOf8),
            ("not all extension_mandatory parameters are supported", ApiErrorKind::ExtensionMandatoryUnsupported),
            ("not all extension_mandatory request options could be met", ApiErrorKind::ExtensionMandatoryUnmet),
            ("not all extension_optional request options handled", ApiErrorKind::ExtensionOptionalNotHandled),
            ("  Size shall be a multiple of 8.  ", ApiErrorKind::SizeNotMultipleOf8),
            ("something else", ApiErrorKind::BadRequest),
        ];
        for (message, kind) in cases {
            let body = format!(r#"{{"message": "{message}"}}"#);
            assert_eq!(err(StatusCode::BAD_REQUEST, &body).kind(), Some(kind), "{message}");
        }
    }

    #[test]
    fn error_details_are_kept() {
        let e = err(
            StatusCode::BAD_REQUEST,
            r#"{"message": "not all extension_mandatory parameters are supported",
                "details": [{"extension_mandatory_unsupported": "abc_route_type is not supported"}]}"#,
        );
        assert_eq!(e.api_error().unwrap().details.as_ref().unwrap().len(), 1);
    }

    #[test]
    fn service_unavailable() {
        let e = err(StatusCode::SERVICE_UNAVAILABLE, r#"{"message": "key data access error"}"#);
        assert_eq!(e.kind(), Some(ApiErrorKind::ServiceUnavailable));
    }

    #[test]
    fn non_json_error_body_is_kept_truncated() {
        let html = format!("<html>{}</html>", "x".repeat(1000));
        let e = err(StatusCode::BAD_GATEWAY, &html);
        match &e {
            Etsi014Error::Api { body: None, raw: Some(raw), .. } => {
                assert_eq!(raw.len(), MAX_RAW_ERROR_BODY);
                assert!(raw.starts_with("<html>"));
            }
            other => panic!("unexpected: {other:?}"),
        }
        assert_eq!(e.kind(), Some(ApiErrorKind::Other));
    }

    #[test]
    fn non_api_errors_have_no_kind() {
        assert_eq!(err(StatusCode::OK, "").kind(), None);
    }
}
