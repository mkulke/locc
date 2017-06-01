#[macro_use]
extern crate serde_derive;

extern crate serde;
extern crate serde_json;
extern crate reqwest;

use std::error::Error;
use std::boxed::Box;

type MaybeUsers = Result<String, Box<Error>>;

#[derive(Deserialize, Debug)]
struct SearchResult {
    name: String,
}

fn get_names(results: Vec<SearchResult>) -> Vec<String> {
    results.into_iter().map(|r| r.name).collect()
}

fn perform() -> MaybeUsers {
    let mut res = reqwest::get("http://jsonplaceholder.typicode.com/users")?;
    let results = res.json::<Vec<SearchResult>>()?;
    let names = get_names(results);
    let joined_names = names.join(", ");
    Ok(joined_names)
}

fn main() {
    match perform() {
        Ok(data) => println!("Ok: {}", data),
        Err(e) => println!("Error: {}", e),
    }
}
