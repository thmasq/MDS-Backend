use actix_files::{NamedFile, Files};
use actix_web::{App, HttpServer, web, HttpResponse, Error};
use meilisearch_sdk::client::Client;
use std::path::PathBuf;
use serde::Deserialize;
use serde::Serialize;

//It was necessary to wrap my query because actix receives it as a serialized JSON file, which needs to be deserialized to be worked with.
//The debug macro was used for the code to be able to pretty print the requests for diagnosing and experimentation.
#[derive(Deserialize, Debug)]
struct SearchQueryWrapper {
    q: String,
}

//Meilisearch doesn't really have a schema like other Databases, but this struct organizes the fields each object in the DB has
//Both the serialize and deserialize macros were used as the Meilisearch SDK required Dese and Actix-web required Serialization to format the responses.
#[derive(Serialize, Deserialize, Debug)]
struct Movie {
    id: i32,
    title: String,
    poster: String,
    overview: String,
    release_date: i64,
}

//This struct wraps the relevant results in a neat way to be used to send responses more efficiently.
#[derive(Serialize, Debug)]
struct SearchResults {
    results: Vec<Movie>,
}

//This is the search function, avaliable at <website_address>/search. It listens for Json requests with a string and returns a response with a JSON file.
//This function is acyncronous, and the main function will call this function as a factory, the threads are disconnected and non blocking.
async fn search(
    query: web::Query<SearchQueryWrapper>,
    client: web::Data<Client>,
) -> Result<HttpResponse, Error> {
    println!("Received search request with query: {:#?}", query);

    //Construct the search query using the Meilisearch SDK builder pattern
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

    //Convert Meilisearch results to your SearchResults struct
    let movies: Vec<Movie> = search_results
        .hits
        .iter()
        .map(|hit| -> Movie {
            //Access the fields from your Meilisearch result, replace these with the actual field names
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

    //Create a SearchResults instance
    let search_results = SearchResults { results: movies };

    println!("Returning search results as JSON: {:#?}", search_results);

    //Return search results as JSON
    Ok(HttpResponse::Ok().json(search_results))
}

//This function serves the main webpage. Like the search function, each time it is called, the main function will spawn a new disconnected thread.
async fn index() -> Result<NamedFile, Error> {
    let path: PathBuf = PathBuf::from("static/index.html"); //Must be replaced with the actual path to your HTML file
    println!("Serving index.html from path: {:?}", path);
    Ok(NamedFile::open(path)?)
}

//For the main function, the actix-rt macro is used to make it easy to scale the backend server. It is basically a factory that spawns new disconnected threads for each function execution.
//This behavior is desired because there really is no advantage of sharing any state between threads for this workload, which reduces atomics overhead. 
#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    //Configured logging for Actix-web, for debugging purposes only. Must be turned off later
    std::env::set_var("RUST_LOG", "actix_web=debug");

    //Uses the SDK to connect to the Meilisearch server. For the prototype I hardcoded the API key
    let meilisearch_client = Client::new(
        "http://localhost:7700",
        Some("OSepughN96MyXGm3wNqaDtCr_tJwzxusvWvkel22NU8"),
    );

    //Creates an instance of an Actix-web http server
    let server = HttpServer::new(move || {
        App::new()
            //I was basing this prototype on an old code snippet I found, which used the deprecated .data function.
            //I still haven't figured out an elegant way of replacing it with the newer app_data function.
            .data(meilisearch_client.clone()) //Share the client across requests
            .service(web::resource("/search").to(search))
            .service(Files::new("/static", "static").show_files_listing())
            .route("/", web::get().to(index))
            .default_service(web::route().to(HttpResponse::NotFound))
    });

    //This binds the server to a specific address and port
    let server = server.bind("127.0.0.1:8080")?;
    println!("Actix-web server started at http://127.0.0.1:8080");
    server.run().await
}
