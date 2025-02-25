use anyhow::Result;
use reqwest::Client;
use serde_json::json;
use std::{fs::File, io::Write, path::Path};

use crate::AudioGeneration;

const OPENAI_AUDIO_API: &str = "https://api.openai.com/v1/audio/speech";

#[async_trait::async_trait]
impl AudioGeneration for crate::AudioGenerator {
    async fn generate_audio(&self, conversation: &str, output_file: &Path) -> Result<()> {
        println!("Generating audio from conversation...");
        let client = Client::new();

        let response = client
            .post(OPENAI_AUDIO_API)
            .header("Authorization", format!("Bearer {}", self.openai_api_key))
            .json(&json!({
                "model": "tts-1",
                "voice": "alloy",
                "input": conversation
            }))
            .send()
            .await?;

        let status = response.status();
        if status.is_success() {
            let audio_content = response.bytes().await?;
            let mut file = File::create(output_file)?;
            file.write_all(&audio_content)?;
            println!("Audio file created: {}", output_file.display());
            Ok(())
        } else {
            let error = response.text().await?;
            Err(anyhow::anyhow!(
                "Failed to generate audio. Status: {}, Error: {}",
                status,
                error
            ))
        }
    }
}
