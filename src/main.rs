use std::{fs::{self, File}, io::Read};

use etsi014::Endpoint;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

    env_logger::init();
    

    let kme_hostname = "10.50.0.21:7443";
    let slave_sae_id = "vKMS";
    
    let buf = fs::read("ESKMCA.crt")?;
    let certs = reqwest::Certificate::from_pem(&buf)?;

    let chain = fs::read("client-chain.pem")?;
    let identity = reqwest::tls::Identity::from_pem(&chain)?;

    let endpoint = Endpoint::new_tls(kme_hostname, slave_sae_id, Some(certs), Some(identity))?;

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