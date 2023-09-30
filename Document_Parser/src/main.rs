use chrono::Datelike;
use fancy_regex::Regex;
use serde_derive::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::error::Error;
use std::path::Path;
use std::process::Command;
use std::result::Result;
use std::{fs, io};

#[derive(Serialize, Deserialize)]
struct Entry {
    id: String,
    title: Option<String>,
    date: Option<i64>,
    content: String,
    link: String,
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

fn return_title(formatted_text: &str, keywords: &[&str]) -> Option<String> {
    let mut found_title: Option<String> = None;
    for line in formatted_text.lines() {
        if keywords.iter().any(|&keyword| line.contains(keyword)) {
            found_title = Some(line.to_string());
            break; // Stop searching once the title is found
        }
    }
    found_title
}

fn return_date(formatted_text: &str) -> Option<i64> {
    let mut found_date: Option<i64> = None;
    for line in formatted_text.lines() {
        if let Some(date) = extract_date(line) {
            found_date = Some(date);
            break; // Stop searching once the date is found
        }
    }
    found_date
}

#[allow(clippy::unnecessary_wraps)]
fn return_parameters(
    text: &str,
    keywords: &[&str],
    existing_titles: &HashSet<String>,
) -> Result<(Option<String>, String, Option<i64>, bool), pdf_extract::OutputError> {
    let found_title = return_title(text, keywords);
    let found_date = return_date(text);

    let mut result_title: Option<String> = found_title.clone();
    let mut result_date: Option<i64> = found_date;
    let mut is_duplicate: bool = false;

    if let Some(ref title) = found_title {
        if existing_titles.contains(title) {
            println!("Warning: Duplicate entry with title '{title}'");
            result_title = None;
            result_date = None;
            is_duplicate = true;
        }
    }

    Ok((result_title, text.to_string(), result_date, is_duplicate))
}

fn extract_date(line: &str) -> Option<i64> {
    let re = Regex::new(r"(\d{2}/\d{2}/\d{4}|\d{2}/\d{2}/\d{2})").expect("Invalid Regular Expression for Date.");
    match re.captures(line) {
        Ok(Some(captures)) => {
            let date_str = captures.get(0)?.as_str();
            let mut parts = date_str.split('/');

            let day: u32 = parts.next()?.parse::<u32>().ok()?;
            let month: u32 = parts.next()?.parse::<u32>().ok()?;
            let year_str = parts.next()?.to_string();

            // Convert year to 4-digit format if it's 2-digit
            let year_2digits: i32 = year_str.parse::<i32>().ok()?;
            let current_year = chrono::Utc::now().year();
            let century = if year_2digits <= (current_year % 100) {
                current_year - (current_year % 100)
            } else {
                current_year - (current_year % 100) - 100
            };
            let year = century + year_2digits;

            let date_time = chrono::NaiveDate::from_ymd_opt(year, month, day)?.and_hms_opt(0, 0, 0);

            date_time.map(|dt| dt.timestamp())
        },
        _ => None,
    }
}

fn get_link(path: &Path) -> Result<String, Box<dyn Error>> {
    Option::map(
        Option::and_then(path.file_stem(), |stem| stem.to_str()),
        |stem_str| -> Result<String, Box<dyn Error>> {
            let parts: Vec<&str> = stem_str.split('_').collect();
            if parts.len() == 2 {
                let id = parts[0];
                let key = parts[1];
                Ok(format!(
                    "https://sig.unb.br/sigrh/downloadArquivo?idArquivo={id}&key={key}"
                ))
            } else {
                Err(Box::from(format!("Invalid filename format or path: {path:?}")))
            }
        },
    )
    .unwrap_or_else(|| Err(Box::from(format!("Invalid filename format or path: {path:?}"))))
}

fn extract_text(path: &Path) -> Result<String, Box<dyn Error>> {
    let output = Command::new("pdftotext")
        .arg("-q") // Suppress output to stderr
        .arg(path)
        .arg("-") // Extract to stdout
        .output()?;

    if output.status.success() {
        let text = String::from_utf8_lossy(&output.stdout);
        Ok(text.into_owned())
    } else {
        Err("pdftotext command failed".into())
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
                &["RESOLUÇÃO", "Cronograma", "Calendário", "Calendario"],
                &existing_titles,
            ) {
                Ok((title, formatted_text, date, is_duplicate)) => {
                    if let Some(title_str) = title.as_ref() {
                        println!("Title found: {title_str}");
                        existing_titles.insert(title_str.clone());

                        let title_hash = {
                            let mut hasher = Sha256::new();
                            hasher.update(title_str);
                            format!("{:x}", hasher.finalize())
                        };

                        // Handle the result of get_link here
                        let link = match get_link(&path) {
                            Ok(link) => link,
                            Err(err) => {
                                eprintln!("Error generating link: {err:?}");
                                continue; // Skip processing this entry and continue with the next one
                            },
                        };

                        let entry = Entry {
                            id: title_hash,
                            title,
                            date,
                            content: formatted_text,
                            link,
                        };

                        entries.push(entry);

                        // Move the PDF to the 'old' folder
                        let old_path =
                            old_folder.join(path.file_name().expect("Couldn't find file name for move operation."));
                        if let Err(err) = fs::rename(&path, &old_path) {
                            eprintln!("Error moving file: {err:?}");
                        }
                    } else if !is_duplicate {
                        println!("No title found");
                    }

                    if !is_duplicate {
                        date.map_or_else(
                            || {
                                println!("No date found");
                            },
                            |date| {
                                println!("Date found: {date}");
                            },
                        );
                    }
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
