use pdf_extract::extract_text;
use fancy_regex::Regex;
use std::path::Path;

fn extract_title<P: AsRef<Path>>(path: P, keywords: &[&str]) -> Result<Option<String>, pdf_extract::OutputError> {
    let text = extract_text(path)?;

    // println!("Original PDF Contents:\n{}", text);

    let formatted_text = format_text(&text);

    println!("Formatted PDF Contents:\n{}", formatted_text);

    for line in formatted_text.lines() {
        if keywords.iter().any(|&keyword| line.contains(keyword)) {
            return Ok(Some(line.to_string()));
        }
    }

    Ok(None)
}

fn format_text(input: &str) -> String {
    let mut formatted_lines = Vec::new();

    input.lines().for_each(|line| {
        // Remove spaces between characters
        let re = Regex::new(r"(?<=\S)\s(?=\S)").unwrap();
        let line_without_spaces = re.replace_all(line, "").to_string();

        // Replace 2 or more spaces with a single space
        let re2 = Regex::new(r"\s{2,}").unwrap();
        let line_with_single_spaces = re2.replace_all(&line_without_spaces, " ").to_string();

        formatted_lines.push(line_with_single_spaces);
    });

    formatted_lines.join("\n")
}

fn main() {
    let pdf_path = "example.pdf"; // Replace with the actual path to your PDF file
    let keywords = ["RESOLUÇÃO", "R E S O L U Ç Ã O", "Header", "Main Title"]; // Add your list of keywords here

    match extract_title(pdf_path, &keywords) {
        Ok(Some(title)) => println!("Title found: {}", title),
        Ok(None) => println!("No title found"),
        Err(err) => eprintln!("Error: {:?}", err),
    }
}
