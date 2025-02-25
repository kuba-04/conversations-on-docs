use anyhow::Result;
use async_trait::async_trait;
use clap::Parser;
use dialoguer::Select;
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[arg(short, long, default_value = "ollama")]
    model_provider: String,
}

pub struct MarkdownProcessor {
    pub input_path: PathBuf,
    pub output_path: PathBuf,
}

struct ConversationGenerator {
    model_type: config::ModelType,
    api_key: String,
    ollama_url: String,
}

struct AudioGenerator {
    openai_api_key: String,
    output_path: PathBuf,
}

// Main processing traits
pub trait MarkdownProcessing {
    fn process_markdown(&self, file_path: &Path) -> Result<String>;
}

#[async_trait]
trait ConversationGeneration {
    async fn generate_conversation(&self, content: &str) -> Result<String>;
}

#[async_trait]
trait AudioGeneration {
    async fn generate_audio(&self, conversation: &str, output_file: &Path) -> Result<()>;
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load configuration
    let config = config::Config::new()?;

    // Ask user to select model provider
    let options = vec!["Ollama", "OpenAI"];
    let selection = Select::new()
        .with_prompt("Choose your model provider")
        .items(&options)
        .default(0)
        .interact()?;

    let model_type = match selection {
        0 => config::ModelType::Ollama(
            std::env::var("OLLAMA_MODEL").expect("OLLAMA_MODEL must be set"),
        ),
        1 => config::ModelType::OpenAI(
            std::env::var("OPENAI_MODEL").expect("OPENAI_MODEL must be set"),
        ),
        _ => unreachable!(),
    };

    println!("Selected: {}", options[selection]);

    // Initialize processors
    let markdown_processor = MarkdownProcessor {
        input_path: config.input.docs_path.clone(),
        output_path: config.output.audio_path.clone(),
    };

    let conversation_generator = ConversationGenerator {
        model_type,
        api_key: config.model.openai_api_key.clone().unwrap_or_default(),
        ollama_url: config
            .model
            .ollama_base_url
            .unwrap_or_else(|| String::from("http://localhost:11434")),
    };

    let audio_generator = AudioGenerator {
        openai_api_key: config
            .model
            .openai_api_key
            .expect("OpenAI API key is required for audio generation"),
        output_path: config.output.audio_path,
    };

    println!("Starting markdown to audio conversion...");

    // Find all markdown files
    let markdown_files = markdown::find_markdown_files(&config.input.docs_path)?;

    println!("Found {} markdown files to process", markdown_files.len());

    for file in markdown_files {
        let audio_filename = file.with_extension("mp3");
        let conv_filename = file.with_extension("conversation.txt");

        // Skip if both conversation and audio files exist
        if audio_filename.exists() && conv_filename.exists() {
            println!("Skipping already processed file: {}", file.display());
            continue;
        }

        println!("Processing: {}", file.display());

        // Read and process markdown
        let content = markdown_processor.process_markdown(&file)?;

        // Limit conversation text length
        let content = if content.len() > 4000 {
            println!(
                "Warning: Truncating content to 4000 characters for {}",
                file.display()
            );
            content.chars().take(4000).collect::<String>()
        } else {
            content
        };

        // Generate conversation if it doesn't exist
        if !conv_filename.exists() {
            let conversation = conversation_generator
                .generate_conversation(&content)
                .await?;
            std::fs::write(&conv_filename, &conversation)?;
            println!("Conversation file created: {}", conv_filename.display());
        }

        // Generate audio if it doesn't exist
        if !audio_filename.exists() {
            let conversation = std::fs::read_to_string(&conv_filename)?;
            audio_generator
                .generate_audio(&conversation, &audio_filename)
                .await?;
        }
    }

    Ok(())
}

// Implementation modules
mod audio;
mod config;
mod conversation;
mod markdown;
