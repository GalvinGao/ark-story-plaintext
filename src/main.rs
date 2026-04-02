use std::fs;
use std::path::Path;

use ark_story_plaintext::parser;
use ark_story_plaintext::renderer;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let input_dir = Path::new("Inputs");
    let output_dir = Path::new("outputs");
    let output_file = output_dir.join("story.txt");

    if !input_dir.exists() {
        eprintln!("Error: Inputs/ directory not found");
        std::process::exit(1);
    }

    fs::create_dir_all(output_dir)?;

    let mut entries: Vec<_> = fs::read_dir(input_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .is_some_and(|ext| ext == "txt")
        })
        .collect();
    entries.sort_by_key(|e| e.file_name());

    let mut parts: Vec<String> = Vec::new();
    for entry in &entries {
        let path = entry.path();
        let content = fs::read_to_string(&path)?;
        let events = parser::parse(&content);
        let markdown = renderer::render(&events);
        parts.push(markdown);
        eprintln!("Processed: {}", path.display());
    }

    let combined = parts.join("\n\n");
    fs::write(&output_file, &combined)?;

    eprintln!("Compiled {} files -> {}", entries.len(), output_file.display());
    Ok(())
}
