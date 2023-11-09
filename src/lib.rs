
use serde::{Deserialize, Serialize};

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug, Default)]
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

impl Endpoint {

    // TODO: IMPORTANT -> Change to https

    pub fn status(&self) -> Result<StatusResponse, Error> {
        let resp = reqwest::blocking::get(format!("http://{}/api/v1/keys/{}/status", self.KME_hostname, self.slave_SAE_ID));
        
        match resp {
            Ok(response) => {
                match response.json::<StatusResponse>() {
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

    pub fn get_key(&self, key_request: Option<&KeyRequest>) -> Result<KeyResponse, Error> {
        let client: reqwest::blocking::Client = reqwest::blocking::Client::new();

        // Create the json body
        let json_body = match key_request {
            Some(request) => request,
            None => &KeyRequest { number: Some(1), size: Some(256), additional_slave_SAE_IDs: None }
        };

        let resp = client.post(format!("http://{}/api/v1/keys/{}/enc_keys", self.KME_hostname, self.slave_SAE_ID))
                    .json(&json_body).send();
        

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

        let resp = client.post(format!("http://{}/api/v1/keys/{}/dec_keys", self.KME_hostname, self.slave_SAE_ID))
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



