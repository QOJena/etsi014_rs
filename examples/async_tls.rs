use std::{fs::{self, File}, io::Read};

use etsi014::Endpoint;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {



    env_logger::init();

    let kme_hostname = "10.50.0.21:7443";
    let slave_sae_id = "vKMS";

    let endpoint = Endpoint::new(kme_hostname, slave_sae_id, Some("ESKMCA.crt".to_string()), Some("client-chain.pem".to_string()))?;

    let status = endpoint.status().await;

    match status {
        Ok(status_response) => {
            println!("Status: {:?}", status_response);
        },
        Err(err) => {
            eprintln!("Error fetching status: {}", err.message);
        }
    }

    Ok(())
}