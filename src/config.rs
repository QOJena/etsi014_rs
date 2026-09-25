use std::path::PathBuf;
use std::{marker::PhantomData, time::Duration};

use log::error;
use reqwest::{Certificate, Identity, Url};

use crate::client::Endpoint;

use crate::error::Etsi014Error as Error;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);

enum Pem {
    Inline(Vec<u8>),
    File(PathBuf)
}

impl Pem {
    fn load(self) -> Result<Vec<u8>, Error> {
        match self {
            Pem::Inline(pem) => Ok(pem),
            Pem::File(path) => std::fs::read(&path).map_err(|source| Error::Io { path, source }),
        }
    }
}


pub struct EndpointBuilder<T> {
    url: String,
    root_ca: Option<Pem>,
    identity: Option<(Pem, Pem)>,
    timeout: Duration,
    allow_insercurre_http: bool,
    _flavor: PhantomData<fn() -> T>
}

impl<T> EndpointBuilder<T> {
    pub(crate) fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            root_ca: None,
            identity: None,
            timeout: DEFAULT_TIMEOUT,
            allow_insercurre_http: false,
            _flavor: PhantomData
        }
    }

    pub fn root_ca(mut self, cert: impl Into<Vec<u8>>) -> Self {
        self.root_ca = Some(Pem::Inline(cert.into()));
        self
    }

    pub fn root_ca_file(mut self, path: impl Into<PathBuf>) -> Self {
        self.root_ca = Some(Pem::File(path.into()));
        self
    }

    pub fn identity_pem(mut self, cert: impl Into<Vec<u8>>, key: impl Into<Vec<u8>>) -> Self {
        self.identity = Some((Pem::Inline(cert.into()), Pem::Inline(key.into())));
        self
    }

    pub fn identity_files(mut self, cert: impl Into<PathBuf>, key: impl Into<PathBuf>) -> Self {
        self.identity = Some((Pem::File(cert.into()), Pem::File(key.into())));
        self
    }

    pub fn timeout(mut self, t: Duration) -> Self {
        self.timeout = t;
        self
    }

    pub fn danger_allow_insecure_http(mut self) -> Self {
        self.allow_insercurre_http = true;
        self
    }

    fn validate_url(&self) -> Result<Url, Error> {
        let url = Url::parse(&self.url)?;
        match url.scheme() {
            "https" => Ok(url),
            "http" if self.allow_insercurre_http => Ok(url),
            other => Err(Error::InsecureScheme(other.into()))
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
                let identity = Identity::from_pem(&pem)
                    .map_err(|e| Error::invalid_pem("client identity", e));
                pem.fill(0);
                Some(identity?)
            }
            None => None,
        };

        Ok((root_ca, identity))
    }
}

impl EndpointBuilder<crate::client::Endpoint> {
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
            url
        })
    }
}

#[cfg(feature = "blocking")]
impl EndpointBuilder<crate::blocking::Endpoint> {
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
            url
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
            let dir = std::env::temp_dir()
                .join(format!("etsi014-{}-{}", name, std::process::id()));
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
        let result = builder().root_ca(cert.as_str()).identity_pem(cert, key).build();
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
            Err(Error::InvalidPem { what: "root CA", .. })
        ));
    }

    #[test]
    fn identity_without_key() {
        let (cert, _) = self_signed();
        let result = builder().identity_pem(cert, "").build();
        assert!(matches!(
            result,
            Err(Error::InvalidPem { what: "client identity", .. })
        ));
    }

    #[test]
    fn identity_without_certificate() {
        let (_, key) = self_signed();
        let result = builder().identity_pem("", key).build();
        assert!(matches!(
            result,
            Err(Error::InvalidPem { what: "client identity", .. })
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
