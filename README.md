# etsi014

A Rust client for the [ETSI GS QKD 014](https://www.etsi.org/deliver/etsi_gs/QKD/001_099/014/01.01.01_60/gs_qkd014v010101p.pdf)
REST API, used by Secure Application Entities (SAEs) to get keys from the
Key Management Entity (KME) of a QKD device.

It covers the three endpoints of the spec:

| Method | ETSI 014 endpoint | Clause |
|---|---|---|
| `Endpoint::status` | `GET /api/v1/keys/{slave_SAE_ID}/status` | 5.2 |
| `Endpoint::get_key` | `POST /api/v1/keys/{slave_SAE_ID}/enc_keys` | 5.3 |
| `Endpoint::get_key_with_id` | `POST /api/v1/keys/{master_SAE_ID}/dec_keys` | 5.4 |

## Installation

```toml
[dependencies]
etsi014 = { git = "https://github.com/<owner>/etsi014_rs" }
```

## Usage

The spec requires HTTPS with mutual TLS: the SAE authenticates to the KME with
a client certificate, and verifies the KME against a trusted CA.

```rust,no_run
use etsi014::{Endpoint, KeyId, KeyIdRequest, KeyRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let endpoint = Endpoint::builder("https://kme-a.example.com")
        .root_ca_file("certs/ca.crt")
        .identity_files("certs/sae_a.crt", "certs/sae_a.key")
        .build()?;

    // SAE A (master) asks its KME for two 256-bit keys shared with SAE B
    let status = endpoint.status("SAE_B").await?;
    println!("{} keys stored", status.stored_key_count);

    let keys = endpoint.get_key("SAE_B", &KeyRequest::new(Some(2), Some(256))).await?;

    // SAE B (slave) later fetches the same keys from its own KME by ID
    let ids = keys.keys.iter().map(|k| KeyId { key_ID: k.key_ID.clone() }).collect();
    let request = KeyIdRequest::new(ids);
    // let same_keys = endpoint_b.get_key_with_id("SAE_A", &request).await?;

    Ok(())
}
```

Key material is returned base64-encoded, as sent by the KME.

### Builder options

| Option | Default | Description |
|---|---|---|
| `root_ca(pem)` | none | CA certificate(s) used to verify the KME, in addition to the system roots |
| `root_ca_file(path)` | none | Same as `root_ca`, read from a file |
| `identity_pem(cert, key)` | none | Client certificate and private key for mutual TLS |
| `identity_files(cert, key)` | none | Same as `identity_pem`, read from files |
| `timeout(Duration)` | 5 s | Request timeout |
| `danger_allow_insecure_http()` | off | Allow `http://` URLs, for tests against a local KME simulator only |

`build()` rejects non-HTTPS URLs unless `danger_allow_insecure_http()` is set.

### Certificates

Certificates and keys are PEM encoded. The `*_pem` methods accept anything
that converts into bytes (`&str`, `String`, `&[u8]`, `Vec<u8>`), which is
handy when they come from environment variables or a secret store:

```rust,ignore
let endpoint = Endpoint::builder("https://kme-a.example.com")
    .root_ca(std::env::var("KME_CA_PEM")?)
    .identity_pem(std::env::var("SAE_CERT_PEM")?, std::env::var("SAE_KEY_PEM")?)
    .build()?;
```

- The root CA may contain several certificates, e.g. a CA bundle.
- The client certificate may be followed by its chain. The private key can
  be PKCS#8, PKCS#1 (RSA) or SEC1 (EC).
- If the certificate and key are in a single PEM, pass it as `cert` and an
  empty `key`: `.identity_pem(combined, "")`.

Files are read, and PEM data parsed, when `build()` is called. It returns
`Etsi014Error::Io` if a file cannot be read, and `Etsi014Error::InvalidPem`
if the root CA contains no certificate or the identity is missing its
certificate or private key.

### Error handling

All methods return `Result<_, Etsi014Error>`. For errors reported by the KME,
`Etsi014Error::kind()` maps the response to the cases defined in the spec:

```rust,ignore
use etsi014::ApiErrorKind;

match endpoint.get_key_with_id("SAE_A", &request).await {
    Ok(keys) => { /* ... */ }
    Err(e) => match e.kind() {
        Some(ApiErrorKind::Unauthorized) => eprintln!("TLS client auth failed or wrong SAE"),
        Some(ApiErrorKind::KeysNotFound) => eprintln!("keys unknown or already retrieved"),
        Some(ApiErrorKind::ServiceUnavailable) => eprintln!("KME error, retry later"),
        _ => eprintln!("{e}"),
    },
}
```

`e.status()` gives the HTTP status, and `e.api_error()` the error body
(clause 6.5) sent by the KME, if any.

Note that a KME removes keys from its pool once they are delivered by
`dec_keys`: if that response is lost (e.g. a timeout), the keys cannot be
fetched again.

## Example

`examples/async_tls.rs` queries the status of a KME and requests a key:

```sh
RUST_LOG=debug cargo run --example async_tls
```

## Development

```sh
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

## License

TODO
