use actix_files::{Files, NamedFile};
use actix_web::{web, App, Error, HttpResponse, HttpServer};
use meilisearch_sdk::client::Client;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

//It was necessary to wrap my query because actix receives it as a serialized JSON file, which
// needs to be deserialized to be worked with. The debug macro was used for the code to be able to
// pretty print the requests for diagnosing and experimentation.
#[derive(Deserialize, Debug)]
struct SearchQueryWrapper {
    q: String,
}

//Meilisearch doesn't really have a schema like other Databases, but this struct organizes the
// fields each object in the DB has Both the serialize and deserialize macros were used as the
// Meilisearch SDK required Dese and Actix-web required Serialization to format the responses.
#[derive(Serialize, Deserialize, Debug)]
struct Movie {
    id: i32,
    title: String,
    poster: String,
    overview: String,
    release_date: i64,
}

//This struct wraps the relevant results in a neat way to be used to send responses more
// efficiently.
#[derive(Serialize, Debug)]
struct SearchResults {
    results: Vec<Movie>,
}

// This function performs a Meilisearch query based on the provided query string and the Meilisearch
// client. The function does not perform any query string trimming itself. The query string should
// be trimmed before calling this function in order to avoid exceeding a certain length.
// The function returns a Result containing Meilisearch search results or an internal server error
// if the query fails.
async fn query_meilisearch(
    query: &str,
    client: &Client,
) -> Result<meilisearch_sdk::search::SearchResults<Movie>, Error> {
    let search_results = client
        .index("movies")
        .search()
        .with_query(query)
        .execute()
        .await
        .map_err(|e| {
            eprintln!("Meilisearch Error: {:?}", e);
            actix_web::error::ErrorInternalServerError("Meilisearch query failed")
        })?;

    Ok(search_results)
}

// This function transforms Meilisearch search results into a custom format suitable for the
// response. It maps Meilisearch hits to a Vec<Movie> and constructs a SearchResults struct for JSON
// serialization.
fn transform_results(search_results: meilisearch_sdk::search::SearchResults<Movie>) -> SearchResults {
    let movies: Vec<Movie> = search_results
        .hits
        .iter()
        .map(|hit| Movie {
            id: hit.result.id.clone(),
            title: hit.result.title.clone(),
            poster: hit.result.poster.clone(),
            overview: hit.result.overview.clone(),
            release_date: hit.result.release_date.clone(),
        })
        .collect();

    SearchResults { results: movies }
}

//This is the search function, avaliable at <website_address>/search. It listens for Json requests
// with a string and returns a response with a JSON file. This function is acyncronous, and the main
// function will call this function as a factory, the threads are disconnected and non blocking.
async fn search(query: web::Query<SearchQueryWrapper>, client: web::Data<Client>) -> Result<HttpResponse, Error> {
    println!("Received search request with query: {:#?}", query);

    // Trim the query to the first 200 characters
    let trimmed_query = &query.q[..200];

    if trimmed_query.len() < 3 {
        // You can adjust the minimum query length
        return Ok(HttpResponse::Ok().json(SearchResults { results: vec![] }));
    }

    // Query Meilisearch
    let search_results = query_meilisearch(trimmed_query, &client).await?;

    println!("Meilisearch search results: {:#?}", search_results);

    // Transform results
    let search_results = transform_results(search_results);

    println!("Returning search results as JSON: {:#?}", search_results);

    Ok(HttpResponse::Ok().json(search_results))
}

//This function serves the main webpage. Like the search function, each time it is called, the main
// function will spawn a new disconnected thread.
async fn index() -> Result<NamedFile, Error> {
    let path: PathBuf = PathBuf::from("static/index.html"); //Must be replaced with the actual path to your HTML file
    println!("Serving index.html from path: {:?}", path);
    Ok(NamedFile::open(path)?)
}

//For the main function, the actix-rt macro is used to make it easy to scale the backend server. It
// is basically a factory that spawns new disconnected threads for each function execution.
// This behavior is desired because there really is no advantage of sharing any state between
// threads for this workload, which reduces atomics overhead.
#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    //Configured logging for Actix-web, for debugging purposes only. Must be turned off later
    std::env::set_var("RUST_LOG", "actix_web=debug");

    //Uses the SDK to connect to the Meilisearch server. For the prototype I hardcoded the API key
    let meilisearch_client = Client::new(
        "http://localhost:7700",
        Some("OSepughN96MyXGm3wNqaDtCr_tJwzxusvWvkel22NU8"),
    );

    let meilisearch_client_data = web::Data::new(meilisearch_client.clone());

    let server = HttpServer::new(move || {
        App::new()
            .app_data(meilisearch_client_data.clone()) // Share the client across requests
            .service(web::resource("/search").to(search))
            .service(Files::new("/static", "static").show_files_listing())
            .route("/", web::get().to(index))
            .default_service(web::route().to(HttpResponse::NotFound))
    });

    let server = server.bind("127.0.0.1:8080")?;
    println!("Actix-web server started at http://127.0.0.1:8080");
    server.run().await
}
