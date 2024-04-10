
// Conditionally import blocking
#[cfg(feature="blocking")]
pub mod blocking;

use serde::{Deserialize, Serialize};

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug, Default, PartialEq)]
pub struct StatusResponse {
    pub source_KME_ID: String,
    pub target_KME_ID: String,
    pub master_SAE_ID: String,
    pub slave_SAE_ID: String,
    pub key_size: u32,
    pub stored_key_count: u32,
    pub max_key_count: u32,
    pub max_key_per_request: u32,
    pub max_key_size: u32,
    pub min_key_size: u32,
    pub max_SAE_ID_count: u32,
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Default)]
pub struct KeyRequest {
    pub number: Option<u32>,
    pub size: Option<u32>,
    pub additional_slave_SAE_IDs: Option<Vec<String>>,
}


#[derive(Serialize, Deserialize, Debug, Default)]
pub struct KeyResponse {
    pub keys: Vec<QKDKey>
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug)]
pub struct QKDKey {
    pub key_ID: String,
    pub key: String,
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize)]
pub struct KeyIdRequest {
    pub key_IDs: Vec<KeyId>
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize)]
pub struct KeyId {
    pub key_ID: String
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug)]
pub struct Error {
    pub message: String,
    pub details: Option<Vec<serde_json::Value>>
}

/**
 * 
 * Defines the ETSI014 endpoint of the QKD device. This allows to make all the etsi request implemented.
 */
#[allow(non_snake_case)]
#[derive(Debug, Clone)]
pub struct Endpoint {
    pub KME_hostname: String,
    pub slave_SAE_ID: String,
}

const SIZE_UUID: usize = 16;
const SIZE_KEY: usize = 32;

pub struct QKDKeyPair {
    pub id: [u8; SIZE_UUID],
    pub key: [u8; SIZE_KEY]
}

impl Error {
    pub fn new(msg: String) -> Self {
        Self {
            message: msg,
            details: None
        }
    }
}

impl KeyRequest {
    pub fn new(number: Option<u32>, size: Option<u32>) -> Self {
        Self {
            number,
            size,
            additional_slave_SAE_IDs: None
        }
    }
}

impl KeyIdRequest {
    pub fn new(key_ids: Vec<KeyId>) -> Self {
        Self {
            key_IDs: key_ids
        }
    }
}

impl Endpoint {

    pub fn new(kme_hostname: &str, slave_sae_id: &str) -> Self {
        Self {
            KME_hostname: String::from(kme_hostname),
            slave_SAE_ID: String::from(slave_sae_id)
        }
    }

    // TODO: IMPORTANT -> Change to https
    pub async fn status(&self) -> Result<StatusResponse, Error> {
        let resp = reqwest::get(format!("http://{}/api/v1/keys/{}/status", self.KME_hostname, self.slave_SAE_ID)).await;
        
        match resp {
            Ok(response) => {
                match response.json::<StatusResponse>().await {
                    Ok(status) => Ok(status),
                    Err(err) => Err(Error {
                        message: err.to_string(),
                        details: None
                    })
                }
            },
            Err(err) => {
                Err(Error {
                    message: err.to_string(),
                    details: None
                })
            }
        }
    }

    pub async fn get_key(&self, key_request: KeyRequest) -> Result<KeyResponse, Error> {
        let client: reqwest::Client = reqwest::Client::new();

        // Create the json body
        // let json_body = match key_request {
        //     Some(request) => request,
        //     None => &KeyRequest { number: Some(1), size: Some(256), additional_slave_SAE_IDs: None }
        // };

        let resp = client.post(format!("http://{}/api/v1/keys/{}/enc_keys", self.KME_hostname, self.slave_SAE_ID))
                    .json(&key_request).send().await;
        

        match resp {
            Ok(response) => {
                if response.status().is_client_error() || response.status().is_server_error() {
                    match response.json::<Error>().await {
                        Ok(error) => Err(error),
                        Err(err) => Err(Error { message: err.to_string(), details: None })
                    }
                } else {
                    match response.json::<KeyResponse>().await {
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

    pub async fn get_key_with_id(&self, key_with_id: &KeyIdRequest) -> Result<KeyResponse, Error> {
        let client = reqwest::Client::new();

        let resp = client.post(format!("http://{}/api/v1/keys/{}/dec_keys", self.KME_hostname, self.slave_SAE_ID))
                    .json(&key_with_id).send().await;

        match resp {
            Ok(response) => {
                if response.status().is_client_error() || response.status().is_server_error() {
                    match response.json::<Error>().await {
                        Ok(error) => Err(error),
                        Err(err) => Err(Error { message: err.to_string(), details: None })
                    }
                } else {
                    match response.json::<KeyResponse>().await {
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

    use crate::{KeyId, KeyIdRequest, KeyRequest, StatusResponse};

    use super::{Endpoint, Error};
    use serde::{Deserialize, Serialize};
    use tokio::test;
    // use tokio_test;

    #[tokio::test] 
    async fn status_test() {
        let endpoint = Endpoint::new("127.0.0.1:8888", "bob");
        
        let status = match endpoint.status().await {
            Ok(ok) => panic!("Should not connect!"),
            Err(_) => return
        };

        
    }

    #[tokio::test] 
    async fn get_key_test() {
        let endpoint = Endpoint::new("127.0.0.1:8888", "bob");

        let request = KeyRequest { number: Some(3), size: Some(256), additional_slave_SAE_IDs: None };

        let keys = match endpoint.get_key(request).await {
            Ok(ok) => panic!("Should not connect!"),
            Err(_) => return
        };

        
    }

    #[tokio::test] 
    async fn get_key_with_id_test() {
        let endpoint = Endpoint::new("127.0.0.1:8888", "bob");
        
        let mut ids = Vec::new();
        ids.push(KeyId {
            key_ID: "0".to_string()
        });

        let request = KeyIdRequest { key_IDs: ids };


        let keys = match endpoint.get_key_with_id(&request).await {
            Ok(ok) => panic!("Should not connect!"),
            Err(_) => return
        };
    }
}

