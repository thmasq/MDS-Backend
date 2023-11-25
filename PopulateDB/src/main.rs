#![allow(non_snake_case)]

use chrono::{DateTime, NaiveDate, Utc};
use clap::Parser;
use crossterm::event::{self, Event, KeyCode};
use crossterm::execute;
use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen};
use rand::Rng;
use serde::{Deserialize, Serialize};
use sqlx::mysql::MySqlConnectOptions;
use sqlx::{query, MySql, Pool};
use std::collections::HashMap;
use std::error::Error;
use std::ffi::OsStr;
use std::fs::File;
use std::io::{stdout, BufReader, Read};
use std::{fmt, fs};

#[derive(Debug)]
struct User {
    email: Option<String>,
    userName: Option<String>,
    token: Option<String>,
}

#[derive(Debug)]
struct Document {
    docName: Option<String>,
    link: Option<String>,
    content: Option<String>,
    docKey: Option<String>,
    creationDate: Option<i64>,
}

#[derive(Debug)]
struct Favorite {
    favoritesId: i64,
    userToken: String,
    documentId: String,
}

#[derive(Debug)]
struct FavoriteItem {
    email: Option<String>,
    docName: Option<String>,
}

#[derive(Parser, Debug, Deserialize)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = "localhost")]
    host: String,

    #[arg(short, long, default_value = "3306")]
    port: u16,

    #[arg(short, long, default_value = "")]
    username: String,

    #[arg(short, long, default_value = "")]
    password: String,

    #[arg(short, long, default_value = "")]
    database: String,
}

#[derive(Serialize, Deserialize)]
struct Entry {
    id: String,
    title: Option<String>,
    date: Option<i64>,
    content: String,
    link: String,
    is_normative: i32,
}

#[derive(Debug)]
enum MyError {
    SqlxError(sqlx::Error),
    IoError(std::io::Error),
    InvalidChoice(String),
}

impl fmt::Display for MyError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::SqlxError(e) => write!(f, "Database error: {e}"),
            Self::InvalidChoice(s) => write!(f, "Invalid choice: {s}"),
            Self::IoError(a) => write!(f, "IO error: {a}"),
        }
    }
}

impl Error for MyError {}

impl From<sqlx::Error> for MyError {
    fn from(err: sqlx::Error) -> Self {
        Self::SqlxError(err)
    }
}

impl From<std::io::Error> for MyError {
    fn from(err: std::io::Error) -> Self {
        Self::IoError(err)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = Args::parse();

    // Check if the config file exists
    if let Ok(mut file) = File::open("populatedb_config.toml") {
        let mut contents = String::new();
        file.read_to_string(&mut contents).expect("Failed to read config file");

        // Parse the TOML file into the Args struct
        let config: Args = toml::from_str(&contents).expect("Failed to parse config file");

        args.username = if args.username.is_empty() {
            config.username
        } else {
            args.username
        };
        args.password = if args.password.is_empty() {
            config.password
        } else {
            args.password
        };
        args.database = if args.database.is_empty() {
            config.database
        } else {
            args.database
        };
    }

    if (
        args.username.is_empty(),
        args.password.is_empty(),
        args.database.is_empty(),
    ) == (true, true, true)
    {
        eprintln!(
            "Error: Username, password, and database must be provided either through command-line arguments or in the config file."
        );
        std::process::exit(1);
    }

    let pool = Pool::connect_with(
        MySqlConnectOptions::new()
            .host(&args.host)
            .port(args.port)
            .username(&args.username)
            .password(&args.password)
            .database(&args.database),
    )
    .await?;

    loop {
        println!();
        println!("1. Create a new document");
        println!("2. Create a new user");
        println!("3. Favorite an item as a user");
        println!("4. Load entries into database");
        println!("5. Exit");

        let choice: i32 = input("Enter your choice: ");

        match choice {
            1 => create_new_document(&pool).await?,
            2 => create_new_user(&pool).await?,
            3 => favorite_item_as_user(&pool).await?,
            4 => populate_users(&pool).await?,
            5 => break,
            _ => println!("Invalid choice. Please enter a valid option."),
        }
    }

    Ok(())
}

async fn create_new_document(pool: &Pool<MySql>) -> Result<(), sqlx::Error> {
    let docName: String = input("Enter document name: ");
    let link: String = input("Enter document link: ");
    let content: String = input("Enter document content: ");
    let docKey: String = input("Enter document key: ");

    // Prompt the user for the creation date
    let creationDate: NaiveDate = input("Enter creation date (YYYY-MM-DD): ");
    let creationDate =
        DateTime::<Utc>::from_naive_utc_and_offset(creationDate.and_hms_opt(0, 0, 0).expect("Impossible date"), Utc);
    let unix_epoch_time = creationDate.timestamp();

    let document = Document {
        docName: Some(docName.clone()),
        link: Some(link.clone()),
        content: Some(content.clone()),
        docKey: Some(docKey.clone()),
        creationDate: Some(unix_epoch_time),
    };

    // Insert the new document into the DOCUMENT table
    let result = sqlx::query!(
        "INSERT INTO DOCUMENT (docName, link, creationDate, content, docKey) VALUES (?, ?, ?, ?, ?)",
        document.docName,
        document.link,
        document.creationDate,
        document.content,
        document.docKey
    )
    .execute(pool)
    .await?;

    if result.rows_affected() > 0 {
        println!("Document created successfully!");
    } else {
        println!("Failed to create document.");
    }

    Ok(())
}

async fn create_new_user(pool: &Pool<MySql>) -> Result<(), sqlx::Error> {
    let email: String = input("Enter user email: ");
    let userName: String = input("Enter user name: ");
    let token: String = input("Enter user token: ");

    let user = User {
        email: Some(email.clone()),
        userName: Some(userName.clone()),
        token: Some(token.clone()),
    };

    // Insert the new user into the USER table
    let result = query!(
        "INSERT INTO USER (email, userName, token) VALUES (?, ?, ?)",
        user.email,
        user.userName,
        user.token
    )
    .execute(pool)
    .await?;

    if result.rows_affected() > 0 {
        println!("User created successfully!");
    } else {
        println!("Failed to create user.");
    }

    Ok(())
}

async fn favorite_item_as_user(pool: &Pool<MySql>) -> Result<(), MyError> {
    let users = list_users(pool).await?;
    let documents = list_documents(pool).await?;

    loop {
        println!();
        println!("1. Create favorite");
        println!("2. List favorites by user");
        println!("3. List users");
        println!("4. List documents");
        println!("5. Exit");

        let choice: i32 = input("Enter your choice: ");
        println!();

        let result = match choice {
            1 => create_favorite(pool, &users, &documents).await,
            2 => list_user_favorites(pool, &input::<String>("Enter a user email")).await,
            3 => print_users(&users),
            4 => print_documents(&documents),
            5 => break,
            _ => {
                println!("\nInvalid choice. Please enter a valid option.");
                Ok(())
            },
        };

        // Propagate any errors
        result?;
    }

    Ok(())
}

async fn create_favorite(
    pool: &Pool<MySql>,
    users: &HashMap<Option<String>, Option<String>>,
    documents: &HashMap<Option<String>, Option<String>>,
) -> Result<(), MyError> {
    let (_, userToken) = select_from_map("Select a user:", users)?;
    let (_, documentId) = select_from_map("Select a document:", documents)?;
    println!();

    // Generate a unique favoritesId
    let mut rng = rand::rngs::OsRng;
    let mut favoritesId: i64;

    // Loop until a unique favoritesId is generated
    loop {
        favoritesId = rng.gen();

        // Check if the generated favoritesId already exists in the database
        let count: i64 = sqlx::query_scalar!("SELECT COUNT(*) FROM favorites WHERE favoritesId = ?", favoritesId)
            .fetch_one(pool)
            .await?;

        if count == 0 {
            break; // Exit the loop if the favoritesId is unique
        }
    }

    let favorite = Favorite {
        favoritesId,
        userToken: userToken.clone().expect("No user token Found."),
        documentId: documentId.clone().expect("No document ID found."),
    };

    // Insert the new favorite into the FAVORITES table
    let result = sqlx::query!(
        "INSERT INTO favorites (favoritesId, userToken, documentId) VALUES (?, ?, ?)",
        favorite.favoritesId,
        favorite.userToken,
        favorite.documentId
    )
    .execute(pool)
    .await?;

    if result.rows_affected() > 0 {
        println!("Favorite added successfully!");
    } else {
        println!("\nFailed to add favorite.");
    }

    Ok(())
}

async fn list_users(pool: &Pool<MySql>) -> Result<HashMap<Option<String>, Option<String>>, MyError> {
    let users = sqlx::query_as!(User, "SELECT userName, email, token FROM USER")
        .fetch_all(pool)
        .await?;

    let mut user_map = HashMap::new();

    for user in users {
        user_map.insert(user.email.clone(), user.token.clone());
    }

    Ok(user_map)
}

async fn list_documents(pool: &Pool<MySql>) -> Result<HashMap<Option<String>, Option<String>>, MyError> {
    let documents = sqlx::query_as!(
        Document,
        "SELECT docName, docKey, content, creationDate, link FROM DOCUMENT"
    )
    .fetch_all(pool)
    .await?;

    let mut document_map = HashMap::new();

    for document in documents {
        document_map.insert(document.docName.clone(), document.docKey.clone());
    }

    Ok(document_map)
}

async fn list_user_favorites(pool: &Pool<MySql>, user_email: &str) -> Result<(), MyError> {
    let favorites = sqlx::query_as!(
        FavoriteItem,
        r#"
		SELECT USER.email, DOCUMENT.docName
		FROM favorites
		INNER JOIN USER ON favorites.userToken = USER.token
		INNER JOIN DOCUMENT ON favorites.documentId = DOCUMENT.docKey
		WHERE USER.email = ?
		ORDER BY USER.email
		"#,
        user_email
    )
    .fetch_all(pool)
    .await?;

    // Convert the vector of FavoriteItem to a vector of tuples
    let favorites_tuples: Vec<(Option<String>, Option<String>)> =
        favorites.into_iter().map(|item| (item.email, item.docName)).collect();

    // Print the favorites and return the result
    print_favorites(&favorites_tuples)
}

fn print_users(users: &HashMap<Option<String>, Option<String>>) -> Result<(), MyError> {
    execute!(stdout(), EnterAlternateScreen)?;

    println!("\nUsers:");
    for (i, user) in users.iter().enumerate() {
        println!("{}. {:?}", i + 1, user.0);
    }
    println!();

    loop {
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key_event) = event::read()? {
                if key_event.code == KeyCode::Char('q') {
                    break;
                }
            }
        }
    }

    execute!(stdout(), LeaveAlternateScreen)?;

    Ok(())
}

fn print_documents(documents: &HashMap<Option<String>, Option<String>>) -> Result<(), MyError> {
    execute!(stdout(), EnterAlternateScreen)?;

    println!("\nDocuments:");
    for (i, document) in documents.iter().enumerate() {
        println!("{}. {:?}", i + 1, document.0);
    }
    println!();

    loop {
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key_event) = event::read()? {
                if key_event.code == KeyCode::Char('q') {
                    break;
                }
            }
        }
    }

    execute!(stdout(), LeaveAlternateScreen)?;

    Ok(())
}

fn print_favorite_item(index: usize, favorite: &(Option<String>, Option<String>)) {
    println!(
        "{}. User Email: {:?}, Document Title: {:?}",
        index + 1,
        favorite.0,
        favorite.1
    );
}

fn print_favorites(favorites: &[(Option<String>, Option<String>)]) -> Result<(), MyError> {
    execute!(stdout(), EnterAlternateScreen)?;

    println!("\nFavorites:");
    for (i, favorite) in favorites.iter().enumerate() {
        print_favorite_item(i, favorite);
    }
    println!();

    loop {
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key_event) = event::read()? {
                if key_event.code == KeyCode::Char('q') {
                    break;
                }
            }
        }
    }

    execute!(stdout(), LeaveAlternateScreen)?;

    Ok(())
}

fn select_from_map<T, U>(prompt: &str, choices: &HashMap<T, U>) -> Result<(T, U), MyError>
where
    T: std::cmp::Eq + std::hash::Hash + std::clone::Clone + std::fmt::Debug,
    U: std::clone::Clone + std::fmt::Debug,
{
    println!("{prompt}");

    for (i, choice) in choices.iter().enumerate() {
        println!("{}. {:?}", i + 1, choice.0);
    }

    println!();
    let index: usize = input("Enter your choice (number): ");

    if index > 0 && index <= choices.len() {
        let selected_key = choices.keys().nth(index - 1).expect("No key");
        let selected_value = choices.get(selected_key).expect("No value");
        Ok((selected_key.clone(), selected_value.clone()))
    } else {
        Err(MyError::InvalidChoice(
            "Invalid choice. Please enter a valid option.".to_string(),
        ))
    }
}

fn input<T>(prompt: &str) -> T
where
    T: std::str::FromStr,
    T::Err: std::fmt::Debug,
{
    loop {
        println!("{prompt}");

        let mut input = String::new();
        std::io::stdin().read_line(&mut input).expect("Failed to read line");

        match input.trim().parse() {
            Ok(value) => return value,
            Err(err) => println!("Error: {err:?}"),
        }
    }
}

async fn populate_users(pool: &Pool<MySql>) -> Result<(), Box<dyn Error>> {
    // Read the JSON file
    let entries = read_json_file()?;

    // Insert the entries into the database
    insert_documents(pool, &entries).await?;

    Ok(())
}

async fn insert_documents(pool: &Pool<MySql>, entries: &[Entry]) -> Result<(), MyError> {
    for entry in entries {
        sqlx::query("INSERT INTO DOCUMENT (docName, link, creationDate, content, docKey) VALUES (?, ?, ?, ?, ?)")
            .bind(&entry.title.clone().unwrap_or_default())
            .bind(&entry.link)
            .bind(entry.date.unwrap_or_default())
            .bind(&entry.content)
            .bind(&entry.id)
            .execute(pool)
            .await?;
    }
    Ok(())
}

fn read_json_file() -> Result<Vec<Entry>, Box<dyn Error>> {
    // List all files in the current directory
    let entries: Vec<_> = fs::read_dir(".")?
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().and_then(OsStr::to_str) == Some("json"))
        .collect();

    // Present the user with a list of options
    for (i, entry) in entries.iter().enumerate() {
        println!("{}. {:?}", i + 1, entry.path().display());
    }

    // Let the user choose a file
    let index: usize = input("Enter your choice (number): ");

    if index > 0 && index <= entries.len() {
        let file = File::open(entries[index - 1].path())?;
        let reader = BufReader::new(file);
        let entries: Vec<Entry> = serde_json::from_reader(reader)?;
        Ok(entries)
    } else {
        Err(Box::new(MyError::InvalidChoice(
            "Invalid choice. Please enter a valid option.".to_string(),
        )))
    }
}
