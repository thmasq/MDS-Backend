use scraper::{Html, Selector};
use std::fs;

fn main() {
    let contents = fs::read("./1.html").expect("Something went wrong reading the file");

    // Convert the bytes to a string, replacing invalid UTF-8 sequences with the lossy replacement
    // character
    let contents_string = String::from_utf8_lossy(&contents);

    let html = Html::parse_document(&contents_string);

    let selector = Selector::parse("a").expect("Could not parse document");

    for element in html.select(&selector) {
        if let Some(href) = element.value().attr("href") {
            if href.starts_with("https://sig.unb.br/sigrh/downloadArquivo?idArquivo=") {
                println!("{}", href);
            }
        }
    }
}
