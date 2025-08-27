


use std::time::Duration;

use crate::{core::{self, build_url}, Error, KeyIdRequest, KeyRequest, KeyResponse, StatusResponse};

use http::{header::USER_AGENT, Request, Response};
use reqwest::Identity;

#[allow(non_snake_case)]
#[derive(Debug, Clone)]
pub struct Endpoint {
    pub KME_hostname: String,
    pub slave_SAE_ID: String,
    pub client: reqwest::blocking::Client,
    tls: bool,
}

impl Endpoint {

    pub fn new(kme_hostname: &str, slave_sae_id: &str, cert: Option<reqwest::Certificate>,  identity: Option<Identity>) -> Result<Self, reqwest::Error> {

        let mut client_builder = reqwest::blocking::Client::builder()
            .user_agent("etsi014-client/0.1.0");

        let tls = cert.is_some() || identity.is_some();

        if let Some(c) = cert {
            client_builder = client_builder.add_root_certificate(c);
        }

        if let Some(id) = identity {
            client_builder = client_builder.identity(id);
        }

        let client = client_builder.build()?;
        
        Ok(Self {
            KME_hostname: String::from(kme_hostname),
            slave_SAE_ID: String::from(slave_sae_id),
            client,
            tls
        })
    }

    fn build_url(&self, path: &str) -> String {
        core::build_url(&self.KME_hostname, self.slave_SAE_ID.as_str(), path, self.tls)
    }

    // TODO: IMPORTANT -> Change to https

    pub fn status(&self) -> Result<StatusResponse, Error> {

        // Add timeout

        let response = self.client.get(self.build_url("status")).timeout(Duration::from_millis(500)).send();

        let response = match response {
            Ok(resp) => {
                if resp.status().is_client_error() || resp.status().is_server_error() {
                    log::error!("Error response: {:?}", resp);
                    match resp.json::<Error>() {
                        Ok(error) => return Err(error),
                        Err(err) => return Err(Error { message: err.to_string(), details: None })
                    }
                }
                resp
            },
            Err(err) => {
            log::error!("Error sending request: {}", err);
                return Err(Error { message: err.to_string(), details: None });
            }
            
        };

        log::debug!("Response: {:?}", response);
        
        match response.json::<StatusResponse>() {
            Ok(status) => Ok(status),
            Err(err) => Err(Error {
                message: err.to_string(),
                details: None
            })
        }
        
    }

    pub fn get_key(&self, key_request: KeyRequest) -> Result<KeyResponse, Error> {
        let client: reqwest::blocking::Client = reqwest::blocking::Client::new();

        // // Create the json body
        // let json_body = match key_request {
        //     Some(request) => request,
        //     None => &KeyRequest { number: Some(1), size: Some(256), additional_slave_SAE_IDs: None }
        // };

        let resp = client.post(self.build_url("enc_keys")).timeout(Duration::from_millis(500))
                    .json(&key_request).send();
        

        match resp {
            Ok(response) => {
                if response.status().is_client_error() || response.status().is_server_error() {
                    match response.json::<Error>() {
                        Ok(error) => Err(error),
                        Err(err) => Err(Error { message: err.to_string(), details: None })
                    }
                } else {
                    match response.json::<KeyResponse>() {
                        Ok(key) => Ok(key),
                        Err(err) => Err(Error { message: err.to_string(), details: None })
                    }
                }
                
            },
            Err(err) => {
                Err(Error { message: err.to_string(), details: None })
            }
        }
    }

    pub fn get_key_with_id(&self, key_with_id: &KeyIdRequest) -> Result<KeyResponse, Error> {
        let client = reqwest::blocking::Client::new();

        let resp = client.post(self.build_url("dec_keys")).timeout(Duration::from_millis(500))
                    .json(&key_with_id).send();

        match resp {
            Ok(response) => {
                if response.status().is_client_error() || response.status().is_server_error() {
                    match response.json::<Error>() {
                        Ok(error) => Err(error),
                        Err(err) => Err(Error { message: err.to_string(), details: None })
                    }
                } else {
                    match response.json::<KeyResponse>() {
                        Ok(key) => Ok(key),
                        Err(err) => Err(Error { message: err.to_string(), details: None })
                    }
                }
                
            },
            Err(err) => {
                Err(Error { message: err.to_string(), details: None })
            }
        }
    }


}



#[cfg(test)]
mod test {
    use crate::{KeyId, KeyIdRequest, KeyRequest};

    use super::Endpoint;

    #[test] 
    fn status_test() {
        let endpoint = Endpoint::new("127.0.0.1:8888", "bob", None, None).unwrap();
        
        let status = match endpoint.status() {
            Ok(ok) => panic!("Should not connect!"),
            Err(_) => return
        };

        
    }

    #[test] 
    fn get_key_test() {
        let endpoint = Endpoint::new("127.0.0.1:8888", "bob", None, None).unwrap();

        let request = KeyRequest { number: Some(3), size: Some(256), additional_slave_SAE_IDs: None };

        let keys = match endpoint.get_key(request) {
            Ok(ok) => panic!("Should not connect!"),
            Err(_) => return
        };

        
    }

    #[test] 
    fn get_key_with_id_test() {
        let endpoint = Endpoint::new("127.0.0.1:8888", "bob", None, None).unwrap();
        
        let mut ids = Vec::new();
        ids.push(KeyId {
            key_ID: "0".to_string()
        });

        let request = KeyIdRequest { key_IDs: ids };


        let keys = match endpoint.get_key_with_id(&request) {
            Ok(ok) => panic!("Should not connect!"),
            Err(_) => return
        };
    }
}


