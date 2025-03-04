use anyhow::Result;
use async_trait::async_trait;
use clap::Parser;
use dialoguer::Select;
use std::path::{Path, PathBuf};
use std::time::Instant;

mod audio_merger;

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

// Add this function after the existing imports
fn generate_intro(file_path: &Path) -> Result<(PathBuf, String)> {
    let chapter_number = file_path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow::anyhow!("Invalid file name"))?;

    let intro_filename = file_path.with_file_name(format!("intro_{}.txt", chapter_number));
    let intro_content = format!("Chapter {}. About NIP-{}.", chapter_number, chapter_number);

    Ok((intro_filename, intro_content))
}

// Add this function to format elapsed time nicely
fn format_elapsed(elapsed: std::time::Duration) -> String {
    let seconds = elapsed.as_secs();
    let minutes = seconds / 60;
    let hours = minutes / 60;
    let seconds_remainder = seconds % 60;
    let minutes_remainder = minutes % 60;

    if hours > 0 {
        format!("{}h {}m {}s", hours, minutes_remainder, seconds_remainder)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, seconds_remainder)
    } else {
        format!("{}.{:03}s", seconds, elapsed.subsec_millis())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let main_start = Instant::now();

    // Load configuration
    let config = config::Config::new()?;

    // Create output directory if it doesn't exist
    std::fs::create_dir_all(&config.output.audio_path)?;

    // Main menu options
    let options = vec![
        "Convert markdown to text conversations",
        "Convert conversations to audio",
        "Generate intros (text and audio)",
        "Merge intro audio with conversation audio",
        "Full process (all steps)",
        "Process specific file",
    ];

    let selection = Select::new()
        .with_prompt("Choose processing mode")
        .items(&options)
        .default(4) // Default to full process
        .interact()?;

    // Find all markdown files
    let markdown_files = markdown::find_markdown_files(&config.input.docs_path)?;
    println!("Found {} markdown files to process", markdown_files.len());

    // For specific file processing
    let files_to_process = if selection == 5 {
        // Create a list of file names for selection
        let file_names: Vec<String> = markdown_files
            .iter()
            .filter_map(|path| path.file_name()?.to_str().map(String::from))
            .collect();

        if file_names.is_empty() {
            return Err(anyhow::anyhow!("No markdown files found"));
        }

        // Let user select a specific file
        let file_selection = Select::new()
            .with_prompt("Choose a file to process")
            .items(&file_names)
            .default(0)
            .interact()?;

        // Find the selected file path
        let selected_file = &file_names[file_selection];
        let selected_path = markdown_files
            .iter()
            .find(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .map_or(false, |name| name == selected_file)
            })
            .ok_or_else(|| anyhow::anyhow!("Selected file not found"))?;

        // Now let user select which operations to perform
        let operation_options = vec![
            "Convert to conversation",
            "Generate audio",
            "Generate intro",
            "Merge audio files",
            "All operations",
        ];

        let operation_selection = Select::new()
            .with_prompt("Choose operation for this file")
            .items(&operation_options)
            .default(4)
            .interact()?;

        // Create a vector with just the selected file
        vec![selected_path.clone()]
    } else {
        // Process all files
        markdown_files
    };

    // Initialize processors
    let model_type =
        if selection == 0 || selection == 4 || (selection == 5 && files_to_process.len() == 1) {
            // Only ask for model if we need conversation generation
            let model_options = vec!["Ollama", "OpenAI"];
            let model_selection = Select::new()
                .with_prompt("Choose your model provider")
                .items(&model_options)
                .default(0)
                .interact()?;

            match model_selection {
                0 => config::ModelType::Ollama(
                    std::env::var("OLLAMA_MODEL").expect("OLLAMA_MODEL must be set"),
                ),
                1 => config::ModelType::OpenAI(
                    std::env::var("OPENAI_MODEL").expect("OPENAI_MODEL must be set"),
                ),
                _ => unreachable!(),
            }
        } else {
            // Default model type for other operations
            config::ModelType::OpenAI("gpt-4o".to_string())
        };

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

    let audio_path = config.output.audio_path.clone();
    let audio_generator = AudioGenerator {
        openai_api_key: config
            .model
            .openai_api_key
            .expect("OpenAI API key is required for audio generation"),
        output_path: audio_path,
    };

    let output_path = config.output.audio_path;

    if selection == 5 {
        // Process specific file with selected operation
        let operation_selection = Select::new()
            .with_prompt("Choose operation for this file")
            .items(&[
                "Convert to conversation",
                "Generate audio",
                "Generate intro",
                "Merge audio files",
                "All operations",
            ])
            .default(4)
            .interact()?;

        match operation_selection {
            0 => {
                generate_conversations(
                    &files_to_process,
                    &markdown_processor,
                    &conversation_generator,
                )
                .await?
            }
            1 => {
                generate_audio_from_conversations(&files_to_process, &output_path, &audio_generator)
                    .await?
            }
            2 => generate_intros(&files_to_process, &output_path, &audio_generator).await?,
            3 => merge_audio_files(&files_to_process, &config.input.docs_path, &output_path)?,
            4 => {
                process_all(
                    &files_to_process,
                    &markdown_processor,
                    &conversation_generator,
                    &audio_generator,
                    &config.input.docs_path,
                    &output_path,
                )
                .await?
            }
            _ => unreachable!(),
        }
    } else {
        // Process all files with selected operation
        match selection {
            0 => {
                generate_conversations(
                    &files_to_process,
                    &markdown_processor,
                    &conversation_generator,
                )
                .await?
            }
            1 => {
                generate_audio_from_conversations(&files_to_process, &output_path, &audio_generator)
                    .await?
            }
            2 => generate_intros(&files_to_process, &output_path, &audio_generator).await?,
            3 => merge_audio_files(&files_to_process, &config.input.docs_path, &output_path)?,
            4 => {
                process_all(
                    &files_to_process,
                    &markdown_processor,
                    &conversation_generator,
                    &audio_generator,
                    &config.input.docs_path,
                    &output_path,
                )
                .await?
            }
            _ => unreachable!(),
        }
    }

    println!(
        "Processing complete! Total time: {}",
        format_elapsed(main_start.elapsed())
    );
    Ok(())
}

// Function to generate conversations from markdown
async fn generate_conversations(
    files: &[PathBuf],
    markdown_processor: &MarkdownProcessor,
    conversation_generator: &ConversationGenerator,
) -> Result<()> {
    println!("Converting markdown to conversations...");
    let start_time = Instant::now();
    let mut processed = 0;

    for file in files {
        let file_start = Instant::now();
        let conv_filename = file.with_extension("conversation.txt");

        if conv_filename.exists() {
            println!(
                "Skipping existing conversation: {}",
                conv_filename.display()
            );
            continue;
        }

        println!("Processing: {}", file.display());
        let content = markdown_processor.process_markdown(file)?;

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

        let conversation = conversation_generator
            .generate_conversation(&content)
            .await?;
        std::fs::write(&conv_filename, &conversation)?;

        let file_elapsed = file_start.elapsed();
        processed += 1;
        println!(
            "Created conversation: {} (took {})",
            conv_filename.display(),
            format_elapsed(file_elapsed)
        );
    }

    let total_elapsed = start_time.elapsed();
    println!(
        "Conversation generation complete! Processed {} files in {}",
        processed,
        format_elapsed(total_elapsed)
    );
    Ok(())
}

// Function to generate audio from conversations
async fn generate_audio_from_conversations(
    files: &[PathBuf],
    output_path: &Path,
    audio_generator: &AudioGenerator,
) -> Result<()> {
    println!("Converting conversations to audio...");

    for file in files {
        let chapter_number = file
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| anyhow::anyhow!("Invalid file name"))?;

        // Check for conversation file in the same directory as the markdown file
        let conv_filename = file.with_extension("conversation.txt");

        // Output audio goes to the output directory
        let audio_filename = output_path.join(format!("{}.mp3", chapter_number));

        println!("Checking: {}", file.display());
        println!("  Conversation file: {}", conv_filename.display());
        println!("  Audio file: {}", audio_filename.display());
        println!("  Conversation exists: {}", conv_filename.exists());
        println!("  Audio exists: {}", audio_filename.exists());

        if !conv_filename.exists() {
            println!("Skipping file without conversation: {}", file.display());
            continue;
        }

        if audio_filename.exists() {
            println!("Skipping existing audio: {}", audio_filename.display());
            continue;
        }

        println!("Generating audio for: {}", conv_filename.display());
        let conversation = std::fs::read_to_string(&conv_filename)?;
        audio_generator
            .generate_audio(&conversation, &audio_filename)
            .await?;
        println!("Created audio: {}", audio_filename.display());
    }

    Ok(())
}

// Function to generate intros
async fn generate_intros(
    files: &[PathBuf],
    output_path: &Path,
    audio_generator: &AudioGenerator,
) -> Result<()> {
    println!("Generating intros...");

    for file in files {
        let chapter_number = file
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| anyhow::anyhow!("Invalid file name"))?;

        let (intro_filename, intro_content) = generate_intro(file)?;
        let intro_audio_filename = output_path.join(format!("intro_{}.mp3", chapter_number));

        if intro_audio_filename.exists() {
            println!(
                "Skipping existing intro audio: {}",
                intro_audio_filename.display()
            );
            continue;
        }

        std::fs::write(&intro_filename, &intro_content)?;
        println!("Created intro text: {}", intro_filename.display());

        audio_generator
            .generate_audio(&intro_content, &intro_audio_filename)
            .await?;
        println!("Created intro audio: {}", intro_audio_filename.display());
    }

    Ok(())
}

// Function to merge audio files
fn merge_audio_files(files: &[PathBuf], input_path: &Path, output_path: &Path) -> Result<()> {
    println!("Merging audio files...");

    for file in files {
        let chapter_number = file
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| anyhow::anyhow!("Invalid file name"))?;

        let intro_audio = input_path.join(format!("intro_{}.mp3", chapter_number));
        let content_audio = input_path.join(format!("{}.mp3", chapter_number));
        let merged_audio = output_path.join(format!("chapter_{}.mp3", chapter_number));

        if merged_audio.exists() {
            println!("Skipping existing merged audio: {}", merged_audio.display());
            continue;
        }

        if !intro_audio.exists() || !content_audio.exists() {
            println!(
                "Skipping merge for chapter {}: missing source files",
                chapter_number
            );
            continue;
        }

        println!("Merging audio for chapter {}", chapter_number);
        audio_merger::merge_audio_files(&intro_audio, &content_audio, &merged_audio)?;
    }

    Ok(())
}

// Function to process all steps
async fn process_all(
    files: &[PathBuf],
    markdown_processor: &MarkdownProcessor,
    conversation_generator: &ConversationGenerator,
    audio_generator: &AudioGenerator,
    input_path: &Path,
    output_path: &Path,
) -> Result<()> {
    let start_time = Instant::now();

    // Generate conversations
    let conv_start = Instant::now();
    generate_conversations(files, markdown_processor, conversation_generator).await?;
    println!(
        "Conversation generation took {}",
        format_elapsed(conv_start.elapsed())
    );

    // Generate audio from conversations
    let audio_start = Instant::now();
    generate_audio_from_conversations(files, output_path, audio_generator).await?;
    println!(
        "Audio generation took {}",
        format_elapsed(audio_start.elapsed())
    );

    // Generate intros
    let intro_start = Instant::now();
    generate_intros(files, output_path, audio_generator).await?;
    println!(
        "Intro generation took {}",
        format_elapsed(intro_start.elapsed())
    );

    // Merge audio files
    let merge_start = Instant::now();
    merge_audio_files(files, input_path, output_path)?;
    println!(
        "Audio merging took {}",
        format_elapsed(merge_start.elapsed())
    );

    let total_elapsed = start_time.elapsed();
    println!(
        "Full processing complete in {}",
        format_elapsed(total_elapsed)
    );
    Ok(())
}

// Implementation modules
mod audio;
mod config;
mod conversation;
mod markdown;
