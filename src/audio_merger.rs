use anyhow::Result;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::process::Command;

pub fn merge_audio_files(intro_path: &Path, content_path: &Path, output_path: &Path) -> Result<()> {
    // Get absolute paths
    let intro_abs = intro_path.canonicalize()?;
    let content_abs = content_path.canonicalize()?;

    // Create a temporary file list for ffmpeg
    let temp_list = output_path.with_extension("txt");
    let mut file = File::create(&temp_list)?;
    writeln!(file, "file '{}'", intro_abs.display())?;
    writeln!(file, "file '{}'", content_abs.display())?;

    // Using ffmpeg with concat demuxer
    let status = Command::new("ffmpeg")
        .arg("-f")
        .arg("concat")
        .arg("-safe")
        .arg("0")
        .arg("-i")
        .arg(&temp_list)
        .arg("-c")
        .arg("copy")
        .arg(output_path)
        .status()?;

    // Clean up the temporary file
    std::fs::remove_file(temp_list)?;

    if !status.success() {
        return Err(anyhow::anyhow!("Failed to merge audio files"));
    }

    println!("Created merged audio: {}", output_path.display());
    Ok(())
}
