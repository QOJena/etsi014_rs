use std::path::PathBuf;
use std::{marker::PhantomData, time::Duration};

use reqwest::{Certificate, Identity, Url};

use crate::error::Etsi014Error as Error;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);

enum Pem {
    Inline(Vec<u8>),
    File(PathBuf),
}

impl Pem {
    fn load(self) -> Result<Vec<u8>, Error> {
        match self {
            Pem::Inline(pem) => Ok(pem),
            Pem::File(path) => std::fs::read(&path).map_err(|source| Error::Io { path, source }),
        }
    }
}

/// Builder for [`Endpoint`](crate::Endpoint) and, with the `blocking`
/// feature, `blocking::Endpoint`.
///
/// Created with [`Endpoint::builder`](crate::Endpoint::builder). Every
/// option is optional; [`build`](EndpointBuilder::build) validates the URL,
/// reads and parses certificates, and creates the HTTP client.
///
/// | Option | Default |
/// |---|---|
/// | [`root_ca`](Self::root_ca) / [`root_ca_file`](Self::root_ca_file) | system roots only |
/// | [`identity_pem`](Self::identity_pem) / [`identity_files`](Self::identity_files) | no client certificate |
/// | [`timeout`](Self::timeout) | 5 seconds |
/// | [`danger_allow_insecure_http`](Self::danger_allow_insecure_http) | HTTPS only |
///
/// # Example
///
/// ```no_run
/// use std::time::Duration;
/// use etsi014::Endpoint;
///
/// let endpoint = Endpoint::builder("https://kme-a.example.com")
///     .root_ca_file("certs/ca.crt")
///     .identity_files("certs/sae_a.crt", "certs/sae_a.key")
///     .timeout(Duration::from_secs(10))
///     .build()?;
/// # Ok::<(), etsi014::Etsi014Error>(())
/// ```
///
/// Certificates can also be passed directly, e.g. when they come from
/// environment variables or a secret store:
///
/// ```no_run
/// use etsi014::Endpoint;
///
/// let endpoint = Endpoint::builder("https://kme-a.example.com")
///     .root_ca(std::env::var("KME_CA_PEM")?)
///     .identity_pem(std::env::var("SAE_CERT_PEM")?, std::env::var("SAE_KEY_PEM")?)
///     .build()?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub struct EndpointBuilder<T> {
    url: String,
    root_ca: Option<Pem>,
    identity: Option<(Pem, Pem)>,
    timeout: Duration,
    allow_insercurre_http: bool,
    _flavor: PhantomData<fn() -> T>,
}

impl<T> EndpointBuilder<T> {
    pub(crate) fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            root_ca: None,
            identity: None,
            timeout: DEFAULT_TIMEOUT,
            allow_insercurre_http: false,
            _flavor: PhantomData,
        }
    }

    /// Trusts the PEM encoded CA certificate(s) in `cert` to verify the KME,
    /// in addition to the system roots.
    ///
    /// `cert` may hold several certificates (a CA bundle). Replaces any
    /// previous call to [`root_ca`](Self::root_ca) or
    /// [`root_ca_file`](Self::root_ca_file).
    ///
    /// # Errors
    ///
    /// [`build`](Self::build) fails with
    /// [`Etsi014Error::InvalidPem`](crate::Etsi014Error::InvalidPem) if `cert`
    /// contains no certificate.
    ///
    /// ```
    /// use etsi014::{Endpoint, Etsi014Error};
    ///
    /// let result = Endpoint::builder("https://kme.example.com")
    ///     .root_ca("not a certificate")
    ///     .build();
    /// assert!(matches!(result, Err(Etsi014Error::InvalidPem { what: "root CA", .. })));
    /// ```
    pub fn root_ca(mut self, cert: impl Into<Vec<u8>>) -> Self {
        self.root_ca = Some(Pem::Inline(cert.into()));
        self
    }

    /// Same as [`root_ca`](Self::root_ca), but reads the PEM from `path`.
    ///
    /// The file is read by [`build`](Self::build), not by this method.
    ///
    /// # Errors
    ///
    /// [`build`](Self::build) fails with
    /// [`Etsi014Error::Io`](crate::Etsi014Error::Io) if the file cannot be read.
    ///
    /// ```
    /// use etsi014::{Endpoint, Etsi014Error};
    ///
    /// let result = Endpoint::builder("https://kme.example.com")
    ///     .root_ca_file("/nonexistent/ca.crt")
    ///     .build();
    /// assert!(matches!(result, Err(Etsi014Error::Io { .. })));
    /// ```
    pub fn root_ca_file(mut self, path: impl Into<PathBuf>) -> Self {
        self.root_ca = Some(Pem::File(path.into()));
        self
    }

    /// Authenticates this SAE to the KME with a PEM encoded client
    /// certificate and private key (mutual TLS).
    ///
    /// - `cert` may be followed by its certificate chain.
    /// - `key` may be PKCS#8, PKCS#1 (RSA) or SEC1 (EC).
    /// - If certificate and key are in a single PEM, pass it as `cert` and an
    ///   empty `key`.
    ///
    /// ```no_run
    /// # use etsi014::Endpoint;
    /// let combined = std::fs::read("certs/sae_a.pem")?;
    /// let endpoint = Endpoint::builder("https://kme-a.example.com")
    ///     .identity_pem(combined, "")
    ///     .build()?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// Replaces any previous call to [`identity_pem`](Self::identity_pem) or
    /// [`identity_files`](Self::identity_files).
    ///
    /// # Errors
    ///
    /// [`build`](Self::build) fails with
    /// [`Etsi014Error::InvalidPem`](crate::Etsi014Error::InvalidPem) if the
    /// certificate or the private key is missing or invalid.
    pub fn identity_pem(mut self, cert: impl Into<Vec<u8>>, key: impl Into<Vec<u8>>) -> Self {
        self.identity = Some((Pem::Inline(cert.into()), Pem::Inline(key.into())));
        self
    }

    /// Same as [`identity_pem`](Self::identity_pem), but reads the
    /// certificate and private key from files.
    ///
    /// The files are read by [`build`](Self::build), not by this method.
    ///
    /// # Errors
    ///
    /// [`build`](Self::build) fails with
    /// [`Etsi014Error::Io`](crate::Etsi014Error::Io) if a file cannot be read,
    /// and as described in [`identity_pem`](Self::identity_pem) otherwise.
    pub fn identity_files(mut self, cert: impl Into<PathBuf>, key: impl Into<PathBuf>) -> Self {
        self.identity = Some((Pem::File(cert.into()), Pem::File(key.into())));
        self
    }

    /// Sets the timeout of each request, from connecting until the response
    /// body has been read. Defaults to 5 seconds.
    ///
    /// Keep in mind that keys fetched with `get_key_with_id` are removed
    /// from the KME once delivered: if the timeout expires while the response
    /// is being read, those keys are lost.
    pub fn timeout(mut self, t: Duration) -> Self {
        self.timeout = t;
        self
    }

    /// Allows `http://` URLs.
    ///
    /// ETSI GS QKD 014 requires HTTPS; without this, [`build`](Self::build)
    /// rejects any other scheme. Only use it against a local KME simulator
    /// in tests: keys would otherwise travel in clear text.
    ///
    /// ```
    /// use etsi014::{Endpoint, Etsi014Error};
    ///
    /// let result = Endpoint::builder("http://localhost:8080").build();
    /// assert!(matches!(result, Err(Etsi014Error::InsecureScheme(_))));
    ///
    /// let endpoint = Endpoint::builder("http://localhost:8080")
    ///     .danger_allow_insecure_http()
    ///     .build()?;
    /// # Ok::<(), Etsi014Error>(())
    /// ```
    pub fn danger_allow_insecure_http(mut self) -> Self {
        self.allow_insercurre_http = true;
        self
    }

    fn validate_url(&self) -> Result<Url, Error> {
        let url = Url::parse(&self.url)?;
        match url.scheme() {
            "https" => Ok(url),
            "http" if self.allow_insercurre_http => Ok(url),
            other => Err(Error::InsecureScheme(other.into())),
        }
    }

    /// Read and parse the root CA certificates and the client identity.
    fn load_tls(&mut self) -> Result<(Vec<Certificate>, Option<Identity>), Error> {
        let root_ca = match self.root_ca.take() {
            Some(pem) => {
                let certs = Certificate::from_pem_bundle(&pem.load()?)
                    .map_err(|e| Error::invalid_pem("root CA", e))?;
                if certs.is_empty() {
                    return Err(Error::InvalidPem {
                        what: "root CA",
                        reason: "no certificate found".into(),
                    });
                }
                certs
            }
            None => Vec::new(),
        };

        let identity = match self.identity.take() {
            Some((cert, key)) => {
                // With rustls, reqwest needs the certificate and key in one buffer
                let mut pem = cert.load()?;
                pem.push(b'\n');
                pem.extend_from_slice(&key.load()?);
                let identity =
                    Identity::from_pem(&pem).map_err(|e| Error::invalid_pem("client identity", e));
                pem.fill(0);
                Some(identity?)
            }
            None => None,
        };

        Ok((root_ca, identity))
    }
}

impl EndpointBuilder<crate::client::Endpoint> {
    /// Validates the configuration and creates the async [`Endpoint`](crate::Endpoint).
    ///
    /// No request is sent to the KME yet.
    ///
    /// # Errors
    ///
    /// - [`Etsi014Error::InvalidUrl`](crate::Etsi014Error::InvalidUrl): the URL cannot be parsed.
    /// - [`Etsi014Error::InsecureScheme`](crate::Etsi014Error::InsecureScheme): the URL is not
    ///   `https` (see [`danger_allow_insecure_http`](Self::danger_allow_insecure_http)).
    /// - [`Etsi014Error::Io`](crate::Etsi014Error::Io): a certificate or key file cannot be read.
    /// - [`Etsi014Error::InvalidPem`](crate::Etsi014Error::InvalidPem): a certificate or key is invalid.
    /// - [`Etsi014Error::Transport`](crate::Etsi014Error::Transport): the HTTP client cannot be created.
    ///
    /// ```
    /// use etsi014::Endpoint;
    ///
    /// let endpoint = Endpoint::builder("https://kme.example.com").build()?;
    /// assert_eq!(endpoint.url.as_str(), "https://kme.example.com/");
    /// # Ok::<(), etsi014::Etsi014Error>(())
    /// ```
    pub fn build(mut self) -> Result<crate::client::Endpoint, Error> {
        let url = self.validate_url()?;
        let (root_ca, identity) = self.load_tls()?;

        let mut client = reqwest::Client::builder()
            .use_rustls_tls()
            .https_only(!self.allow_insercurre_http)
            .timeout(self.timeout);

        for ca in root_ca {
            client = client.add_root_certificate(ca);
        }

        if let Some(id) = identity {
            client = client.identity(id);
        }

        Ok(crate::client::Endpoint {
            client: client.build()?,
            url,
        })
    }
}

#[cfg(feature = "blocking")]
impl EndpointBuilder<crate::blocking::Endpoint> {
    /// Validates the configuration and creates the [`blocking::Endpoint`](crate::blocking::Endpoint).
    ///
    /// No request is sent to the KME yet.
    ///
    /// # Errors
    ///
    /// - [`Etsi014Error::InvalidUrl`](crate::Etsi014Error::InvalidUrl): the URL cannot be parsed.
    /// - [`Etsi014Error::InsecureScheme`](crate::Etsi014Error::InsecureScheme): the URL is not
    ///   `https` (see [`danger_allow_insecure_http`](Self::danger_allow_insecure_http)).
    /// - [`Etsi014Error::Io`](crate::Etsi014Error::Io): a certificate or key file cannot be read.
    /// - [`Etsi014Error::InvalidPem`](crate::Etsi014Error::InvalidPem): a certificate or key is invalid.
    /// - [`Etsi014Error::Transport`](crate::Etsi014Error::Transport): the HTTP client cannot be created.
    ///
    /// ```
    /// use etsi014::blocking::Endpoint;
    ///
    /// let endpoint = Endpoint::builder("https://kme.example.com").build()?;
    /// # Ok::<(), etsi014::Etsi014Error>(())
    /// ```
    pub fn build(mut self) -> Result<crate::blocking::Endpoint, Error> {
        let url = self.validate_url()?;
        let (root_ca, identity) = self.load_tls()?;

        let mut client = reqwest::blocking::Client::builder()
            .use_rustls_tls()
            .https_only(!self.allow_insercurre_http)
            .timeout(self.timeout);

        for ca in root_ca {
            client = client.add_root_certificate(ca);
        }

        if let Some(id) = identity {
            client = client.identity(id);
        }

        Ok(crate::blocking::Endpoint {
            client: client.build()?,
            url,
        })
    }
}

#[cfg(test)]
mod test {
    use std::path::Path;

    use rcgen::{CertificateParams, KeyPair};

    use super::*;

    /// Self-signed certificate and its private key, both PEM encoded.
    fn self_signed() -> (String, String) {
        let key = KeyPair::generate().unwrap();
        let cert = CertificateParams::new(vec!["sae.example.com".into()])
            .unwrap()
            .self_signed(&key)
            .unwrap();
        (cert.pem(), key.serialize_pem())
    }

    fn builder() -> EndpointBuilder<crate::client::Endpoint> {
        EndpointBuilder::new("https://kme.example.com")
    }

    /// Temporary directory holding `cert.pem` and `key.pem`, removed on drop.
    struct TempPems(PathBuf);

    impl TempPems {
        fn new(name: &str, cert: &str, key: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("etsi014-{}-{}", name, std::process::id()));
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join("cert.pem"), cert).unwrap();
            std::fs::write(dir.join("key.pem"), key).unwrap();
            Self(dir)
        }

        fn cert(&self) -> PathBuf {
            self.0.join("cert.pem")
        }

        fn key(&self) -> PathBuf {
            self.0.join("key.pem")
        }
    }

    impl Drop for TempPems {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn no_certificates() {
        assert!(builder().build().is_ok());
    }

    #[test]
    fn inline_pem() {
        let (cert, key) = self_signed();
        let result = builder()
            .root_ca(cert.as_str())
            .identity_pem(cert, key)
            .build();
        assert!(result.is_ok());
    }

    #[test]
    fn root_ca_bundle() {
        let (first, _) = self_signed();
        let (second, _) = self_signed();
        let result = builder().root_ca(first + &second).build();
        assert!(result.is_ok());
    }

    #[test]
    fn combined_identity_pem() {
        let (cert, key) = self_signed();
        let result = builder().identity_pem(cert + &key, "").build();
        assert!(result.is_ok());
    }

    #[test]
    fn pem_files() {
        let (cert, key) = self_signed();
        let files = TempPems::new("pem_files", &cert, &key);
        let result = builder()
            .root_ca_file(files.cert())
            .identity_files(files.cert(), files.key())
            .build();
        assert!(result.is_ok());
    }

    #[test]
    fn missing_root_ca_file() {
        let path = Path::new("/nonexistent/ca.pem");
        let result = builder().root_ca_file(path).build();
        assert!(matches!(result, Err(Error::Io { path: p, .. }) if p == path));
    }

    #[test]
    fn missing_key_file() {
        let (cert, key) = self_signed();
        let files = TempPems::new("missing_key_file", &cert, &key);
        let missing = files.0.join("missing.pem");
        let result = builder().identity_files(files.cert(), &missing).build();
        assert!(matches!(result, Err(Error::Io { path, .. }) if path == missing));
    }

    #[test]
    fn root_ca_without_certificate() {
        let result = builder().root_ca("not a certificate").build();
        assert!(matches!(
            result,
            Err(Error::InvalidPem {
                what: "root CA",
                ..
            })
        ));
    }

    #[test]
    fn identity_without_key() {
        let (cert, _) = self_signed();
        let result = builder().identity_pem(cert, "").build();
        assert!(matches!(
            result,
            Err(Error::InvalidPem {
                what: "client identity",
                ..
            })
        ));
    }

    #[test]
    fn identity_without_certificate() {
        let (_, key) = self_signed();
        let result = builder().identity_pem("", key).build();
        assert!(matches!(
            result,
            Err(Error::InvalidPem {
                what: "client identity",
                ..
            })
        ));
    }

    #[test]
    fn url_is_checked_before_certificates() {
        let result = EndpointBuilder::<crate::client::Endpoint>::new("http://kme.example.com")
            .root_ca("not a certificate")
            .build();
        // The URL is validated first, so the scheme error wins
        assert!(matches!(result, Err(Error::InsecureScheme(_))));
    }

    #[cfg(feature = "blocking")]
    #[test]
    fn blocking_build_loads_certificates() {
        let (cert, key) = self_signed();
        let result = EndpointBuilder::<crate::blocking::Endpoint>::new("https://kme.example.com")
            .root_ca(cert.as_str())
            .identity_pem(cert, key)
            .build();
        assert!(result.is_ok());

        let result = EndpointBuilder::<crate::blocking::Endpoint>::new("https://kme.example.com")
            .root_ca("not a certificate")
            .build();
        assert!(matches!(result, Err(Error::InvalidPem { .. })));
    }
}
