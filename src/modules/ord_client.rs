use anyhow::{Result, Context};
use reqwest::Client;
use serde_json::Value;
use std::time::Duration;

pub struct OrdClient {
    client: Client,
    base_url: String,
}

impl OrdClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("Failed to build HTTP client"),
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }

    /// Fetches block details including inscriptions and transactions
    pub async fn get_block(&self, height: u64) -> Result<Value> {
        let url = format!("{}/block/{}", self.base_url, height);
        self.get_json(&url).await
    }

    /// Fetches specific inscription metadata
    pub async fn get_inscription(&self, inscription_id: &str) -> Result<Value> {
        let url = format!("{}/inscription/{}", self.base_url, inscription_id);
        self.get_json(&url).await
    }

    /// Fetches raw inscription content as a string
    pub async fn get_content(&self, inscription_id: &str) -> Result<String> {
        let url = format!("{}/content/{}", self.base_url, inscription_id);
        let resp = self.client.get(url)
            .send()
            .await?
            .error_for_status()?;
        
        Ok(resp.text().await?)
    }

    /// Fetches the current block height from ord
    pub async fn get_block_height(&self) -> Result<u64> {
        let url = format!("{}/blockheight", self.base_url);
        let resp = self.client.get(&url)
            .send()
            .await
            .context("Failed to fetch block height")?
            .error_for_status()
            .context("Block height request returned error status")?;

        let text = resp.text().await.context("Failed to read block height response")?;
        let height = text.trim().parse::<u64>().context("Failed to parse block height")?;
        Ok(height)
    }

    async fn get_json(&self, url: &str) -> Result<Value> {
        let resp = self.client.get(url)
            .header("Accept", "application/json")
            .send()
            .await
            .context(format!("Failed to send request to {}", url))?;

        let status = resp.status();
        if !status.is_success() {
            anyhow::bail!("Request failed with status: {} for URL: {}", status, url);
        }

        resp.json::<Value>().await.context("Failed to parse JSON")
    }
}