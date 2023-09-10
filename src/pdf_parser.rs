use pdf::object::*;
use pdf::file::File;
use pdf::primitive::*;
use pdf::error::PdfError;
use std::collections::HashMap;

//
fn extract_title(file: &File<Vec<u8>>) -> Result<String, PdfError> {
    for page in file.pages() {
        let title = page.page_dict
            .get("Contents")
            .and_then(|page_refs| file.resolve(page_refs.clone()).ok())
            .and_then(|contents| file.get_stream(contents).ok())
            .and_then(|contents| {
                let text_content = extract_text_from_content(&contents).ok()?;
                find_title_in_text(&text_content)
            });
    
        if let Some(title) = title {
            return Ok(title);
        }
    }
    
    Err(PdfError::Custom("Title not found".to_string()))
}

fn extract_text_from_content(contents: &Stream) -> Result<String, PdfError> {
    // Extract and concatenate text from content streams
    let mut text_content = String::new();
    for token in contents.decode()? {
        if let Token::Operator(op) = token {
            text_content.push_str(&op.text);
            text_content.push(' ');
        }
    }
    Ok(text_content)
}

fn find_title_in_text(text: &str) -> Option<String> {
    // Split the text into lines
    let lines: Vec<&str> = text.lines().collect();

    // Still to define appropriate heuristics for identifying titles
    let font_size_threshold = 14.0; // Adjust this threshold as needed
    let alignment_pattern = r"^\s*(?:center|right|left|justify)";
    let keyword_patterns = ["Chapter", "Section", "Title", "Chapter"];
    let special_characters = [':', '-', '|']; // Add more if needed

    for line in lines {
        // Check font size and style
        if line.contains("font-size:") {
            let font_size = extract_font_size(line);
            if font_size > font_size_threshold {
                return Some(line.to_string());
            }
        }

        // Check text alignment
        if regex::is_match(line, alignment_pattern).is_ok() {
            return Some(line.to_string());
        }

        // Check for keywords
        for keyword in &keyword_patterns {
            if line.contains(keyword) {
                return Some(line.to_string());
            }
        }

        // Check for special characters
        for character in &special_characters {
            if line.contains(*character) {
                return Some(line.to_string());
            }
        }
    }

    None
}

fn main() {
    let file = File::<Vec<u8>>::open("example.pdf").unwrap();

    match extract_title(&file) {
        Ok(title) => println!("Title: {}", title),
        Err(e) => eprintln!("Error: {:?}", e),
    }
}