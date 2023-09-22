use std::io;
use std::fs;
use std::path::Path;
use serde_json::json;
use std::error::Error;
use fancy_regex::Regex;
use pdf_extract::extract_text;

fn create_folders_if_not_exist() -> Result<(), io::Error> {
    let folders = ["in", "out", "old"];

    for folder in folders.iter() {
        let folder_path = Path::new(folder);
        if !folder_path.exists() {
            println!("The '{}' folder doesn't exist. Create it? (Y/n)", folder);
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;

            if input.trim().to_lowercase() == "y" {
                fs::create_dir(folder_path)?;
                println!("Created '{}' folder.", folder);
            } else {
                println!("Folder '{}' does not exist. Exiting.", folder);
                std::process::exit(1);
            }
        }
    }

    Ok(())
}

fn return_parameters<P: AsRef<Path>>(
    path: P,
    keywords: &[&str],
) -> Result<(Option<String>, String), pdf_extract::OutputError> {
    let text = extract_text(&path)?;
    let formatted_text = format_text(&text);

    println!("Formatted PDF Contents:\n{}", formatted_text);

    let mut found_title = None;
    for line in formatted_text.lines() {
        if keywords.iter().any(|&keyword| line.contains(keyword)) {
            found_title = Some(line.to_string());
            break; // Stop searching once the title is found
        }
    }

    Ok((found_title, formatted_text))
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

fn main() -> Result<(), Box<dyn Error>> {
    create_folders_if_not_exist()?;
    let in_folder = Path::new("in");
    let out_folder = Path::new("out");
    let old_folder = Path::new("old");

    for entry in fs::read_dir(in_folder)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() && path.extension().map_or(false, |ext| ext == "pdf") {
            match return_parameters(&path, &["RESOLUÇÃO", "R E S O L U Ç Ã O", "Header", "Main Title"]) {
                Ok((title, formatted_text)) => {
                    if let Some(ref title_str) = title {
                        println!("Title found: {}", title_str);
                    } else {
                        println!("No title found");
                    }

                    // Create a JSON file with the same name as the PDF in the 'out' folder
                    let json_file_name = path.with_extension("json");
                    let json_path = out_folder.join(json_file_name.file_name().unwrap());

                    let json_data = json!({
                        "title": title,
                        "content": formatted_text,
                    });

                    let json_str = serde_json::to_string_pretty(&json_data)?;

                    fs::write(&json_path, json_str)?;
                    // Move the PDF to the 'old' folder
                    let old_path = old_folder.join(path.file_name().unwrap());
                    fs::rename(&path, old_path)?;
                }
                Err(err) => {
                    eprintln!("Error: {:?}", err);
                }
            }
        }
    }
    Ok(())
}
