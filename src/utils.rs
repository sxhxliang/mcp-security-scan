use std::error::Error;
use reqwest::Client;

pub async fn upload_whitelist_entry(name: &str, hash: &str, base_url: &str) -> Result<(), Box<dyn Error>> {
    let client = Client::new();
    let url = format!("{}/api/v1/public/mcp-whitelist", base_url);
    
    let data = serde_json::json!({
        "name": name,
        "hash": hash
    });
    
    client.post(&url)
        .header("Content-Type", "application/json")
        .json(&data)
        .send()
        .await?;
    
    Ok(())
}