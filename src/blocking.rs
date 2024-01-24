


use std::time::Duration;

use crate::{StatusResponse, Error, KeyRequest, KeyResponse, KeyIdRequest};

use http::{Request,Response};

#[allow(non_snake_case)]
#[derive(Debug, Clone)]
pub struct Endpoint {
    pub KME_hostname: String,
    pub slave_SAE_ID: String,
}

impl Endpoint {

    // TODO: IMPORTANT -> Change to https

    pub fn status(&self) -> Result<StatusResponse, Error> {

        // Add timeout

        let client = reqwest::blocking::Client::new();

        let response = client.get(format!("http://{}/api/v1/keys/{}/status", self.KME_hostname, self.slave_SAE_ID)).timeout(Duration::from_millis(500)).send().map_err(|e| Error::new(e.to_string()))?;
        
        match response.json::<StatusResponse>() {
            Ok(status) => Ok(status),
            Err(err) => Err(Error {
                message: err.to_string(),
                details: None
            })
        }
        
    }

    pub fn get_key(&self, key_request: Option<&KeyRequest>) -> Result<KeyResponse, Error> {
        let client: reqwest::blocking::Client = reqwest::blocking::Client::new();

        // Create the json body
        let json_body = match key_request {
            Some(request) => request,
            None => &KeyRequest { number: Some(1), size: Some(256), additional_slave_SAE_IDs: None }
        };

        let resp = client.post(format!("http://{}/api/v1/keys/{}/enc_keys", self.KME_hostname, self.slave_SAE_ID)).timeout(Duration::from_millis(500))
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

        let resp = client.post(format!("http://{}/api/v1/keys/{}/dec_keys", self.KME_hostname, self.slave_SAE_ID)).timeout(Duration::from_millis(500))
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





