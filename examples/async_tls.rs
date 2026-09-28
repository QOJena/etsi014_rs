use etsi014::{Endpoint, KeyRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let endpoint = Endpoint::builder("https://kme.example.com")
        .build()
        .unwrap();

    let status = endpoint.status("alice").await;

    match status {
        Ok(status_response) => {
            println!("Status: {:?}", status_response);
        }
        Err(err) => {
            eprintln!("Error fetching status: {}", err);
        }
    }

    let key = endpoint
        .get_key("alice", &KeyRequest::new(Some(1), Some(256)))
        .await;
    match key {
        Ok(key_response) => {
            println!("Key: {:?}", key_response);
        }
        Err(err) => {
            eprintln!("Error fetching key: {}", err);
        }
    }

    Ok(())
}
