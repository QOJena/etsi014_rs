use std::{marker::PhantomData, time::Duration};

use log::error;
use reqwest::{Certificate, Identity, Url};

use crate::client::Endpoint;

use crate::error::Etsi014Error as Error;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);

pub struct EndpointBuilder<T> {
    url: String,
    root_ca: Option<Certificate>,
    identity: Option<Identity>,
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

    pub fn root_ca(mut self, cert: Certificate) -> Self {
        self.root_ca = Some(cert);
        self
    }

    pub fn identity(mut self, id: Identity) -> Self {
        self.identity = Some(id);
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

}

impl EndpointBuilder<crate::client::Endpoint> {
    pub fn build(self) -> Result<crate::client::Endpoint, Error> {
        let url = self.validate_url()?;
        let mut client = reqwest::Client::builder()
            .use_rustls_tls()
            .https_only(!self.allow_insercurre_http)
            .timeout(self.timeout);

        if let Some(ca) = self.root_ca {
            client = client.add_root_certificate(ca);
        }

        if let Some(id) = self.identity {
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
    pub fn build(self) -> Result<crate::blocking::Endpoint, Error> {
        let url = self.validate_url()?;
        let mut client = reqwest::blocking::Client::builder()
            .use_rustls_tls()
            .https_only(!self.allow_insercurre_http)
            .timeout(self.timeout);

        if let Some(ca) = self.root_ca {
            client = client.add_root_certificate(ca);
        }

        if let Some(id) = self.identity {
            client = client.identity(id);
        }

        Ok(crate::blocking::Endpoint {
            client: client.build()?,
            url
        })
    }
}
