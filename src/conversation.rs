// Will implement conversation generation later

use anyhow::Result;
use async_trait::async_trait;
use ollama_rs::{generation::completion::request::GenerationRequest, Ollama as OllamaRs};
use serde_json::json;

use crate::{config::ModelType, ConversationGeneration, ConversationGenerator};

pub struct ConversationPrompt {
    pub system: String,
    pub user: String,
}

impl Default for ConversationPrompt {
    fn default() -> Self {
        Self {
            system: "You are an expert at converting technical documentation into natural conversations between a student and a teacher. Keep the technical accuracy but make it engaging and easier to understand. IMPORTANT: Output should have at most 4096 characters. It is also important to not include any json or code blocks in the output. ".into(),
            user: "Convert the following markdown documentation into a natural conversation between two
             Software Developers, first named Jaf is an expert in the protocol we are talking about,
             a second named Paul is a frontend developer who is new to this protocol. 
             Preserve all technical information but make it more engaging:".into(),
        }
    }
}

#[async_trait]
impl ConversationGeneration for ConversationGenerator {
    async fn generate_conversation(&self, content: &str) -> Result<String> {
        let prompt = ConversationPrompt::default();

        match &self.model_type {
            ModelType::Ollama(model) => {
                println!("Making Ollama API call...");
                let ollama = OllamaRs::default();
                let request = GenerationRequest::new(model.clone(), content.to_string())
                    .system(prompt.system);

                println!("Sending request to Ollama...");
                match ollama.generate(request).await {
                    Ok(response) => {
                        println!("Received response from Ollama");
                        Ok(response.response)
                    }
                    Err(e) => {
                        println!("Error from Ollama: {}", e);
                        Err(e.into())
                    }
                }
            }
            ModelType::OpenAI(model) => {
                println!("Making OpenAI API call...");
                let client = reqwest::Client::new();

                let payload = json!({
                    "model": model,
                    "messages": [
                        {
                            "role": "system",
                            "content": prompt.system
                        },
                        {
                            "role": "user",
                            "content": format!("{}\n\n{}", prompt.user, content)
                        }
                    ],
                    "temperature": 0.7,
                    "max_tokens": 2000
                });

                println!("Sending request to OpenAI...");
                let response: serde_json::Value = client
                    .post("https://api.openai.com/v1/chat/completions")
                    .header("Authorization", format!("Bearer {}", self.api_key))
                    .header("Content-Type", "application/json")
                    .json(&payload)
                    .send()
                    .await?
                    .json()
                    .await?;

                let answer = response["choices"][0]["message"]["content"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Invalid response from OpenAI"))?
                    .to_string();

                println!("Received response from OpenAI");
                Ok(answer)
            }
        }
    }
}
