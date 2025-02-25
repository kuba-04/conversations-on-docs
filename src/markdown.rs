// Will implement markdown processing later

use crate::{MarkdownProcessing, MarkdownProcessor};
use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub fn find_markdown_files(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut markdown_files = Vec::new();

    for entry in WalkDir::new(dir)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.extension().map_or(false, |ext| ext == "md") {
            markdown_files.push(path.to_path_buf());
        }
    }

    Ok(markdown_files)
}

impl MarkdownProcessing for MarkdownProcessor {
    fn process_markdown(&self, file_path: &Path) -> Result<String> {
        fs::read_to_string(file_path).map_err(Into::into)
    }
}
