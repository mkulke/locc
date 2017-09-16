#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;
extern crate reqwest;
#[macro_use]
extern crate clap;
extern crate geo;
extern crate num_traits;
extern crate rand;

mod locc;

use std::process;
use clap::{Arg, App, SubCommand};
use std::io::Write;
use locc::{handle_dis, handle_rev, handle_rnd, handle_loc, handle_bbox};

fn is_float(x: String) -> Result<(), String> {
    match x.parse::<f64>() {
        Ok(_) => Ok(()),
        Err(_) => Err("Value is not a proper float.".to_string()),
    }
}

fn is_positive_float(x: String) -> Result<(), String> {
    match x.parse::<f64>() {
            Ok(f) => Ok(f),
            Err(_) => Err("Value is not a proper float.".to_string()),
        }
        .and_then(|f| match f < 0. {
                      false => Ok(()),
                      true => Err("Value is not positive.".to_string()),
                  })
}

fn get_loc_arg<'a, 'b>(name: &'a str, short: &str) -> Arg<'a, 'b> {
    Arg::with_name(name)
        .required(true)
        .require_delimiter(true)
        .short(short)
        .long(name)
        .value_name("lon,lat")
        .validator(is_float)
        .number_of_values(2)
        .takes_value(true)
}

fn get_place_arg<'a, 'b>(name: &'a str, short: &str) -> Arg<'a, 'b> {
    Arg::with_name(name)
        .required(true)
        .short(short)
        .long(name)
        .value_name("place name")
        .number_of_values(1)
        .takes_value(true)
}

fn get_cli_app<'a, 'b>() -> App<'a, 'b> {
    App::new("Location utility")
        .version(crate_version!())
        .author(crate_authors!())
        .subcommand(SubCommand::with_name("bbox")
                        .about("Retrieve a bbox via search string")
                        .arg(get_place_arg("place", "P"))
                        .arg(Arg::with_name("edge length")
                                 .required(false)
                                 .short("L")
                                 .long("length")
                                 .value_name("edge length in km")
                                 .default_value("1")
                                 .validator(is_positive_float)
                                 .number_of_values(1)
                                 .takes_value(true)))
        .subcommand(SubCommand::with_name("loc")
                        .about("Retrieve a location via search string")
                        .arg(get_place_arg("place", "P")))
        .subcommand(SubCommand::with_name("rev")
                        .about("Reverse geocode a place via location")
                        .arg(get_loc_arg("location", "L")))
        .subcommand(SubCommand::with_name("rnd")
                        .about("Get a random location within a km radius")
                        .arg(get_loc_arg("location", "L")
                                 .conflicts_with("place")
                                 .required_unless("place"))
                        .arg(get_place_arg("place", "P")
                                 .conflicts_with("location")
                                 .required(false))
                        .arg(Arg::with_name("radius")
                                 .required(true)
                                 .short("R")
                                 .long("radius")
                                 .value_name("radius in km")
                                 .validator(is_positive_float)
                                 .number_of_values(1)
                                 .takes_value(true)))
        .subcommand(SubCommand::with_name("dis")
                        .about("Return the distance between two points in meters")
                        .arg(get_loc_arg("location1", "1"))
                        .arg(get_loc_arg("location2", "2")))
}

fn bail_out(message: String) {
    writeln!(&mut std::io::stderr(), "{}", message).expect("could not write to stderr");
    process::exit(1);
}

fn main() {
    let app = get_cli_app();
    let matches = app.get_matches();

    let subcommand_result = match matches.subcommand() {
        ("loc", Some(sub_m)) => handle_loc(sub_m),
        ("bbox", Some(sub_m)) => handle_bbox(sub_m),
        ("rnd", Some(sub_m)) => handle_rnd(sub_m),
        ("rev", Some(sub_m)) => handle_rev(sub_m),
        ("dis", Some(sub_m)) => handle_dis(sub_m),
        _ => Err(From::from(matches.usage())),
    };

    match subcommand_result {
        Ok(result) => println!("{}", result),
        Err(err) => bail_out(err.to_string()),
    }
}
