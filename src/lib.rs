//! Client for the [ETSI GS QKD 014] key delivery REST API.
//!
//! ETSI GS QKD 014 defines how a *Secure Application Entity* (SAE), i.e. an
//! application that consumes quantum keys, obtains them from the *Key
//! Management Entity* (KME) of its local QKD device. This crate implements
//! the SAE side of that API.
//!
//! | Method | ETSI 014 endpoint | Clause |
//! |---|---|---|
//! | [`Endpoint::status`] | `GET /api/v1/keys/{slave_SAE_ID}/status` | 5.2 |
//! | [`Endpoint::get_key`] | `POST /api/v1/keys/{slave_SAE_ID}/enc_keys` | 5.3 |
//! | [`Endpoint::get_key_with_id`] | `POST /api/v1/keys/{master_SAE_ID}/dec_keys` | 5.4 |
//!
//! # Master and slave SAEs
//!
//! Keys are always shared between two SAEs, each connected to its own KME:
//!
//! 1. The **master** SAE (`SAE_A`) asks its KME for new keys with
//!    [`Endpoint::get_key`], naming the slave SAE (`SAE_B`). It receives the
//!    key material together with a key ID for each key.
//! 2. The master sends the key IDs (never the keys) to the slave over any
//!    channel of its choice.
//! 3. The **slave** SAE asks *its own* KME for the same keys by ID with
//!    [`Endpoint::get_key_with_id`], naming the master SAE.
//!
//! ```text
//!  SAE_A ── get_key("SAE_B") ──────────▶ KME_A
//!    │                                     ║ QKD link
//!    │ key IDs                             ║
//!    ▼                                     ║
//!  SAE_B ── get_key_with_id("SAE_A") ──▶ KME_B
//! ```
//!
//! # Quick start
//!
//! The spec requires HTTPS with mutual TLS (clause 5.1): the SAE
//! authenticates with a client certificate and verifies the KME against a
//! trusted CA. Both are configured on the [`EndpointBuilder`].
//!
//! ```no_run
//! use etsi014::{Endpoint, KeyId, KeyIdRequest, KeyRequest};
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), etsi014::Etsi014Error> {
//! // On SAE_A (master)
//! let kme_a = Endpoint::builder("https://kme-a.example.com")
//!     .root_ca_file("certs/ca.crt")
//!     .identity_files("certs/sae_a.crt", "certs/sae_a.key")
//!     .build()?;
//!
//! let status = kme_a.status("SAE_B").await?;
//! println!("{} keys available for SAE_B", status.stored_key_count);
//!
//! let response = kme_a.get_key("SAE_B", &KeyRequest::new(Some(2), Some(256))).await?;
//! let ids: Vec<KeyId> = response
//!     .keys
//!     .iter()
//!     .map(|k| KeyId { key_ID: k.key_ID.clone() })
//!     .collect();
//! // ... send the key IDs to SAE_B ...
//!
//! // On SAE_B (slave)
//! let kme_b = Endpoint::builder("https://kme-b.example.com")
//!     .root_ca_file("certs/ca.crt")
//!     .identity_files("certs/sae_b.crt", "certs/sae_b.key")
//!     .build()?;
//!
//! let same_keys = kme_b.get_key_with_id("SAE_A", &KeyIdRequest::new(ids)).await?;
//! assert_eq!(same_keys.keys.len(), 2);
//! # Ok(())
//! # }
//! ```
//!
//! Key material is returned exactly as sent by the KME: base64-encoded, in
//! [`QKDKey::key`].
//!
//! # Error handling
//!
//! Every request returns `Result<_, `[`Etsi014Error`]`>`. Errors reported by
//! the KME itself can be matched against the cases defined in the spec with
//! [`Etsi014Error::kind`]:
//!
//! ```no_run
//! use etsi014::{ApiErrorKind, Endpoint, KeyIdRequest};
//!
//! # async fn run(endpoint: Endpoint, request: KeyIdRequest) {
//! match endpoint.get_key_with_id("SAE_A", &request).await {
//!     Ok(response) => println!("got {} keys", response.keys.len()),
//!     Err(e) => match e.kind() {
//!         Some(ApiErrorKind::Unauthorized) => eprintln!("TLS client auth failed or wrong SAE"),
//!         Some(ApiErrorKind::KeysNotFound) => eprintln!("keys unknown or already retrieved"),
//!         Some(ApiErrorKind::ServiceUnavailable) => eprintln!("KME error, retry later"),
//!         _ => eprintln!("{e}"),
//!     },
//! }
//! # }
//! ```
//!
//! # Cargo features
//!
//! - `blocking`: enables `blocking::Endpoint`, a synchronous client with
//!   the same API, for applications that do not use an async runtime.
//!
//! # Logging
//!
//! Non-success responses from the KME are logged at `debug` level through
//! the [`log`](https://docs.rs/log) facade.
//!
//! [ETSI GS QKD 014]: https://www.etsi.org/deliver/etsi_gs/QKD/001_099/014/01.01.01_60/gs_qkd014v010101p.pdf
#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_docs)]

#[cfg(feature = "blocking")]
#[cfg_attr(docsrs, doc(cfg(feature = "blocking")))]
pub mod blocking;

mod client;
mod config;
mod core;
mod error;
mod key;
mod models;

pub use client::Endpoint;
pub use config::EndpointBuilder;
pub use error::{ApiError, ApiErrorKind, Etsi014Error};
pub use key::{QKDKey, QKDKeyPair};
pub use models::{KeyId, KeyIdRequest, KeyRequest, KeyResponse, StatusResponse};
