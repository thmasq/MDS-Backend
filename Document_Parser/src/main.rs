use fancy_regex::Regex;
use pdf_extract::extract_text;
use serde_json::json;
use std::error::Error;
use std::path::Path;
use std::{fs, io};

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

fn return_parameters(
    text: &str,
    keywords: &[&str],
) -> Result<(Option<String>, String, Option<i64>), pdf_extract::OutputError> {
    let formatted_text = format_text(&text);

    println!("Formatted PDF Contents:\n{}", formatted_text);

    let mut found_title = None;
    let mut found_date = None;
    for line in formatted_text.lines() {
        if keywords.iter().any(|&keyword| line.contains(keyword)) {
            found_title = Some(line.to_string());
        }
        if let Some(date) = extract_date(&line) {
            found_date = Some(date);
        }
        if found_title.is_some() && found_date.is_some() {
            break; // Stop searching once both title and date are found
        }
    }

    Ok((found_title, formatted_text, found_date))
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

fn extract_date(line: &str) -> Option<i64> {
    let re = Regex::new(r"(\d{2})/(\d{2})/(\d{4})").unwrap();
    if let Ok(Some(captures)) = re.captures(line) {
        let day = captures.get(1)?.as_str().parse::<u32>().ok()?;
        let month = captures.get(2)?.as_str().parse::<u32>().ok()?;
        let year = captures.get(3)?.as_str().parse::<i32>().ok()?;
        let date = chrono::NaiveDate::from_ymd_opt(year, month, day)?;
        Some(date.and_hms_opt(0, 0, 0)?.timestamp())
    } else {
        None
    }
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
            let text = extract_text(&path)?;
            match return_parameters(&text, &["RESOLUÇÃO", "R E S O L U Ç Ã O", "Header", "Main Title"]) {
                Ok((title, formatted_text, date)) => {
                    if let Some(ref title_str) = title {
                        println!("Title found: {}", title_str);
                    } else {
                        println!("No title found");
                    }

                    if let Some(date) = date {
                        println!("Date found: {}", date);
                    } else {
                        println!("No date found");
                    }

                    // Create a JSON file with the same name as the PDF in the 'out' folder
                    let json_file_name = path.with_extension("json");
                    let json_path = out_folder.join(json_file_name.file_name().unwrap());

                    let json_data = json!({
                        "title": title,
                        "date": date,
                        "content": formatted_text,
                    });

                    let json_str = serde_json::to_string_pretty(&json_data)?;

                    fs::write(&json_path, json_str)?;
                    // Move the PDF to the 'old' folder
                    let old_path = old_folder.join(path.file_name().unwrap());
                    fs::rename(&path, old_path)?;
                },
                Err(err) => {
                    eprintln!("Error: {:?}", err);
                },
            }
        }
    }
    Ok(())
}
