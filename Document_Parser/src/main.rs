use fancy_regex::Regex;
use pdf_extract::extract_text;
use serde_derive::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::error::Error;
use std::path::Path;
use std::{fs, io};

#[derive(Serialize, Deserialize)]
struct Entry {
    id: String,
    title: Option<String>,
    date: Option<i64>,
    content: String,
    old_file_name: String,
}

#[derive(Serialize, Deserialize)]
struct Data {
    entries: Vec<Entry>,
}

fn create_folders_if_not_exist() -> Result<(), io::Error> {
    let folders: [&str; 3] = ["in", "out", "old"];

    for folder in &folders {
        let folder_path = Path::new(folder);
        if !folder_path.exists() {
            println!("The '{folder}' folder doesn't exist. Create it? (Y/n)");
            let mut input: String = String::new();
            io::stdin().read_line(&mut input)?;

            if input.trim().to_lowercase() == "y" {
                fs::create_dir(folder_path)?;
                println!("Created '{folder}' folder.");
            } else {
                println!("Folder '{folder}' does not exist. Exiting.");
                std::process::exit(1);
            }
        }
    }

    Ok(())
}

#[allow(clippy::unnecessary_wraps)]
fn return_parameters(
    text: &str,
    keywords: &[&str],
    existing_titles: &HashSet<String>,
) -> Result<(Option<String>, String, Option<i64>), pdf_extract::OutputError> {
    let formatted_text: String = format_text(text);

    println!("Formatted PDF Contents:\n{formatted_text}");

    let mut found_title: Option<String> = None;
    let mut found_date: Option<i64> = None;
    for line in formatted_text.lines() {
        if keywords.iter().any(|&keyword| line.contains(keyword)) {
            found_title = Some(line.to_string());
        }
        if let Some(date) = extract_date(line) {
            found_date = Some(date);
        }
        if found_title.is_some() && found_date.is_some() {
            break; // Stop searching once both title and date are found
        }
    }

    if let Some(ref title) = found_title {
        if existing_titles.contains(title) {
            println!("Warning: Duplicate entry with title '{title}'");
            found_title = None;
            found_date = None;
        }
    }

    Ok((found_title, formatted_text, found_date))
}

fn format_text(input: &str) -> String {
    let mut formatted_lines: Vec<String> = Vec::new();

    input.lines().for_each(|line| {
        // Remove spaces between characters
        let re: Regex = Regex::new(r"(?<=\S)\s(?=\S)").expect("Invalid Lookaround Regular Expression.");
        let line_without_spaces: String = re.replace_all(line, "").to_string();

        // Replace 2 or more spaces with a single space
        let re2: Regex = Regex::new(r"\s{2,}").expect("Invalid Regular Expression for looking for repeated spaces.");
        let line_with_single_spaces: String = re2.replace_all(&line_without_spaces, " ").to_string();

        formatted_lines.push(line_with_single_spaces);
    });

    formatted_lines.join("\n")
}

fn extract_date(line: &str) -> Option<i64> {
    let re = Regex::new(r"(\d{2})/(\d{2})/(\d{4})").expect("Invalid Regular Expression for Date.");
    match re.captures(line) {
        Ok(Some(captures)) => {
            let day: u32 = captures.get(1)?.as_str().parse::<u32>().ok()?;
            let month: u32 = captures.get(2)?.as_str().parse::<u32>().ok()?;
            let year: i32 = captures.get(3)?.as_str().parse::<i32>().ok()?;
            let date: chrono::NaiveDate = chrono::NaiveDate::from_ymd_opt(year, month, day)?;
            Some(date.and_hms_opt(0, 0, 0)?.timestamp())
        },
        _ => None,
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    create_folders_if_not_exist()?;
    let in_folder = Path::new("in");
    let out_folder = Path::new("out");
    let old_folder = Path::new("old");

    let mut existing_titles: HashSet<String> = HashSet::new();
    let mut entries: Vec<Entry> = Vec::new();

    // Load existing entries from entries.json if it exists
    let json_path = out_folder.join("entries.json");
    if json_path.exists() {
        let json_str: String = fs::read_to_string(&json_path)?;
        let existing_entries: Data = serde_json::from_str(&json_str)?;
        for entry in existing_entries.entries {
            if let Some(ref title) = entry.title {
                existing_titles.insert(title.clone());
            }
            entries.push(entry);
        }
    }

    for entry in fs::read_dir(in_folder)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() && path.extension().map_or(false, |ext| ext == "pdf") {
            let text = extract_text(&path)?;
            match return_parameters(
                &text,
                &["RESOLUÇÃO", "R E S O L U Ç Ã O", "Header", "Main Title"],
                &existing_titles,
            ) {
                Ok((title, formatted_text, date)) => {
                    title.as_ref().map_or_else(
                        || {
                            println!("No title found");
                        },
                        |title_str| {
                            println!("Title found: {title_str}");
                            existing_titles.insert(title_str.clone());
                        },
                    );

                    date.map_or_else(
                        || {
                            println!("No date found");
                        },
                        |date| {
                            println!("Date found: {date}");
                        },
                    );

                    let title_hash = title.as_ref().map_or_else(String::new, |title| {
                        let mut hasher = Sha256::new();
                        hasher.update(title);
                        format!("{:x}", hasher.finalize())
                    });

                    let entry = Entry {
                        id: title_hash,
                        title,
                        date,
                        content: formatted_text,
                        old_file_name: path
                            .file_name()
                            .expect("Couldn't get file name for entry.")
                            .to_string_lossy()
                            .into_owned(),
                    };

                    entries.push(entry);

                    // Move the PDF to the 'old' folder
                    let old_path =
                        old_folder.join(path.file_name().expect("Couldn't find file name for move operation."));
                    fs::rename(&path, old_path)?;
                },
                Err(err) => {
                    eprintln!("Error: {err:?}");
                },
            }
        }
    }

    let data: Data = Data { entries };

    let json_str: String = serde_json::to_string_pretty(&data)?;

    fs::write(&json_path, json_str)?;

    Ok(())
}
