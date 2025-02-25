use anyhow::{anyhow, Result};
use dotenv::dotenv;
use serde::Deserialize;
use std::fmt;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub input: InputConfig,
    pub model: ModelConfig,
    pub output: OutputConfig,
}

#[derive(Debug, Deserialize)]
pub struct InputConfig {
    pub docs_path: PathBuf,
}

#[derive(Debug, Deserialize)]
pub struct ModelConfig {
    pub model_type: ModelType,
    pub openai_api_key: Option<String>,
    pub ollama_base_url: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum ModelType {
    Ollama(String),
    OpenAI(String),
}

impl fmt::Display for ModelType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ModelType::Ollama(model) => write!(f, "{}", model),
            ModelType::OpenAI(model) => write!(f, "{}", model),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct OutputConfig {
    pub audio_path: PathBuf,
}

impl Config {
    pub fn new() -> Result<Self> {
        dotenv().ok();

        // First try to load from environment variables
        if let Ok(config) = Self::from_env() {
            return Ok(config);
        }

        // If env vars not set, try to load from config file
        Self::from_file()
    }

    fn from_env() -> Result<Self> {
        let model_type = if let Ok(model) = std::env::var("OPENAI_MODEL") {
            ModelType::OpenAI(model)
        } else if let Ok(model) = std::env::var("OLLAMA_MODEL") {
            ModelType::Ollama(model)
        } else {
            return Err(anyhow!("No model configuration found in environment"));
        };

        Ok(Config {
            input: InputConfig {
                docs_path: PathBuf::from(std::env::var("DOCS_PATH")?),
            },
            model: ModelConfig {
                model_type,
                openai_api_key: std::env::var("OPENAI_API_KEY").ok(),
                ollama_base_url: std::env::var("OLLAMA_BASE_URL").ok(),
            },
            output: OutputConfig {
                audio_path: PathBuf::from(std::env::var("AUDIO_OUTPUT_PATH")?),
            },
        })
    }

    fn from_file() -> Result<Self> {
        let config_paths = vec![
            "config.toml",
            "Config.toml",
            "~/.config/markdown-to-audio/config.toml",
        ];

        for path in config_paths {
            if let Ok(content) = std::fs::read_to_string(path) {
                return Ok(toml::from_str(&content)?);
            }
        }

        Err(anyhow!("No configuration file found"))
    }
}
