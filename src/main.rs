use actix_files::{NamedFile, Files};
use actix_web::{App, HttpServer, web, HttpResponse, Error};
use meilisearch_sdk::client::Client;
use std::path::PathBuf;
use serde::Deserialize;
use serde::Serialize;

#[derive(Deserialize, Debug)]
struct SearchQueryWrapper {
    q: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct Movie {
    id: i32,
    title: String,
    poster: String,
    overview: String,
    release_date: i64,
}

#[derive(Serialize, Debug)]
struct SearchResults {
    results: Vec<Movie>,
}

async fn search(
    query: web::Query<SearchQueryWrapper>,
    client: web::Data<Client>,
) -> Result<HttpResponse, Error> {
    println!("Received search request with query: {:#?}", query);

    // Construct the search query using the Meilisearch SDK builder pattern
    let search_results: meilisearch_sdk::search::SearchResults<Movie>  = client
        .index("movies")
        .search()
        .with_query(&query.q)
        .execute()
        .await
        .map_err(|e| {
            eprintln!("Meilisearch Error: {:?}", e);
            actix_web::error::ErrorInternalServerError("Meilisearch query failed")
        })?;

    println!("Meilisearch search results: {:#?}", search_results);

    // Convert Meilisearch results to your SearchResults struct
    let movies: Vec<Movie> = search_results
        .hits
        .iter()
        .map(|hit| -> Movie {
            // Access the fields from your Meilisearch result, replace these with the actual field names
            Movie {
                id: hit.result.id.clone(),
                title: hit.result.title.clone(),
                poster: hit.result.poster.clone(),
                overview: hit.result.overview.clone(),
                release_date: hit.result.release_date.clone(),
            }
        })
        .collect();

    println!("Converted movies: {:#?}", movies);

    // Create a SearchResults instance
    let search_results = SearchResults { results: movies };

    println!("Returning search results as JSON: {:#?}", search_results);

    // Return search results as JSON
    Ok(HttpResponse::Ok().json(search_results))
}

async fn index() -> Result<NamedFile, Error> {
    let path: PathBuf = PathBuf::from("static/index.html"); // Replace with the actual path to your HTML file
    println!("Serving index.html from path: {:?}", path);
    Ok(NamedFile::open(path)?)
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    // Configure logging for Actix-web
    std::env::set_var("RUST_LOG", "actix_web=debug");

    // Initialize the Meilisearch client
    let meilisearch_client = Client::new(
        "http://localhost:7700",
        Some("OSepughN96MyXGm3wNqaDtCr_tJwzxusvWvkel22NU8"),
    );

    // Create an Actix-web server
    let server = HttpServer::new(move || {
        App::new()
            .data(meilisearch_client.clone()) // Share the client across requests
            .service(web::resource("/search").to(search))
            .service(Files::new("/static", "static").show_files_listing())
            .route("/", web::get().to(index))
            .default_service(web::route().to(HttpResponse::NotFound))
    });

    // Bind and run the server
    let server = server.bind("127.0.0.1:8080")?;
    println!("Actix-web server started at http://127.0.0.1:8080");
    server.run().await
}
