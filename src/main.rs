#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;
extern crate reqwest;
#[macro_use]
extern crate clap;

use std::error::Error;
use std::process;
use std::boxed::Box;
use clap::{Arg, App, SubCommand, ArgMatches};
use std::fmt;
use std::fmt::Display;
use reqwest::header::UserAgent;
use std::io::Write;

static NOMINATIM_ENDPOINT: &'static str = "http://nominatim.openstreetmap.org";
const VERSION: &'static str = env!("CARGO_PKG_VERSION");
const NAME: &'static str = env!("CARGO_PKG_NAME");

#[derive(Deserialize, Debug)]
struct Location {
    lat: String,
    lon: String,
}

impl Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{},{}", self.lon, self.lat)
    }
}

#[derive(Deserialize, Debug)]
struct Place {
    display_name: String,
}

impl Display for Place {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.display_name)
    }
}


#[derive(Debug)]
struct NotFoundError;

impl Display for NotFoundError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "No result found")
    }
}

impl Error for NotFoundError {
    fn description(&self) -> &str {
        "No result found"
    }

    fn cause(&self) -> Option<&Error> {
        None
    }
}

fn get_query_string(params: Vec<(&str, &str)>) -> String {
    let pairs: Vec<String> = params
        .into_iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect();
    pairs.join("&")
}

fn reverse_geocode(lon: &str, lat: &str) -> Result<Place, Box<Error>> {
    let params = vec![("format", "json"), ("lon", lon), ("lat", lat)];
    let query_string = get_query_string(params);
    let url = format!("{}/reverse?{}", NOMINATIM_ENDPOINT, query_string);
    let client = reqwest::Client::new()?;
    let mut res = client
        .get(&url)
        .header(UserAgent(format!("{} v{}", NAME, VERSION)))
        .send()?;
    let result = res.json::<Place>()?;
    Ok(result)
}

fn search(place_name: &str) -> Result<Location, Box<Error>> {
    let params = vec![("format", "jsonv2"), ("q", place_name), ("limit", "1")];
    let query_string = get_query_string(params);
    let url = format!("{}/search?{}", NOMINATIM_ENDPOINT, query_string);
    let client = reqwest::Client::new()?;
    let mut res = client
        .get(&url)
        .header(UserAgent(format!("{} v{}", NAME, VERSION)))
        .send()?;
    let mut results = res.json::<Vec<Location>>()?;
    results.reverse();
    let first = results.pop();
    match first {
        Some(r) => Ok(r),
        _ => Err(Box::new(NotFoundError)),
    }
}

fn is_float(x: String) -> Result<(), String> {
    match x.parse::<f64>() {
        Ok(_) => Ok(()),
        Err(_) => Err("Value is not a proper float.".to_string()),
    }
}

fn get_cli_app<'a, 'b>() -> App<'a, 'b> {
    App::new("Location utility")
        .version(crate_version!())
        .author(crate_authors!())
        .subcommand(SubCommand::with_name("loc")
                        .about("Retrieve a location via search string")
                        .arg(Arg::with_name("place")
                                 .required(true)
                                 .short("P")
                                 .long("place")
                                 .value_name("place name")
                                 .number_of_values(1)
                                 .takes_value(true)))
        .subcommand(SubCommand::with_name("rev")
                        .about("Reverse geocode a place via location")
                        .arg(Arg::with_name("location")
                                 .required(true)
                                 .require_delimiter(true)
                                 .short("L")
                                 .long("location")
                                 .value_name("lon,lat")
                                 .validator(is_float)
                                 .number_of_values(2)
                                 .takes_value(true)))
}

fn bail_out(message: &str) {
    writeln!(&mut std::io::stderr(), "{}", message).expect("could not write to stderr");
    process::exit(1);
}

fn handle_rev(matches: ArgMatches) {
    let mut parts: Vec<&str> = matches
        .subcommand_matches("rev")
        .and_then(|m| m.values_of("location"))
        .unwrap()
        .collect();
    let lat = parts.pop().unwrap();
    let lon = parts.pop().unwrap();
    match reverse_geocode(lon, lat) {
        Ok(data) => println!("{}", data),
        Err(e) => bail_out(e.description()),
    }
}

fn handle_loc(matches: ArgMatches) {
    let place = matches
        .subcommand_matches("loc")
        .and_then(|m| m.value_of("place"))
        .unwrap();
    match search(place) {
        Ok(data) => println!("{}", data),
        Err(e) => bail_out(e.description()),
    }
}

fn main() {
    let app = get_cli_app();
    let matches = app.get_matches();

    match matches.subcommand_name() {
        Some("loc") => handle_loc(matches),
        Some("rev") => handle_rev(matches),
        _ => bail_out(matches.usage()),
    }
}
