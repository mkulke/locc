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

use std::error::Error;
use std::process;
use std::boxed::Box;
use clap::{Arg, App, SubCommand, ArgMatches, Values};
use std::fmt;
use std::fmt::Display;
use reqwest::header::UserAgent;
use std::io::Write;
use geo::Point;
use geo::haversine_distance::HaversineDistance;
use rand::distributions::{IndependentSample, Range};

static NOMINATIM_ENDPOINT: &str = "http://nominatim.openstreetmap.org";
const VERSION: &str = env!("CARGO_PKG_VERSION");
const NAME: &str = env!("CARGO_PKG_NAME");
const EARTH_RADIUS_KM: f64 = 6373.;

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
    let first = results.pop().ok_or("No result found")?;
    Ok(first)
}

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
                        .arg(get_loc_arg("location", "L")))
        .subcommand(SubCommand::with_name("rnd")
                        .about("Get a random location within a km radius")
                        .arg(get_loc_arg("location", "L"))
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

fn bail_out(message: &str) {
    writeln!(&mut std::io::stderr(), "{}", message).expect("could not write to stderr");
    process::exit(1);
}

fn handle_rev(matches: &ArgMatches) {
    let mut parts: Vec<&str> = matches.values_of("location").unwrap().collect();
    let lat = parts.pop().unwrap();
    let lon = parts.pop().unwrap();
    match reverse_geocode(lon, lat) {
        Ok(data) => println!("{}", data),
        Err(e) => bail_out(e.description()),
    }
}

fn get_random_point(center: &Point<f64>, radius: f64) -> Point<f64> {
    let mut rng = rand::thread_rng();
    let dist_range = Range::new(0., radius);
    let distance = dist_range.ind_sample(&mut rng);
    let bearing_range = Range::new(0., 360.);
    let bearing = bearing_range.ind_sample(&mut rng);

    direction(center, bearing, distance)
}

#[cfg(test)]
mod get_random_point {
    use super::*;
    use geo::contains::Contains;
    use geo::{LineString, Polygon};
    use std::fs::File;

    #[derive(Deserialize, Debug)]
    struct TestCoordinates(f64, f64);

    #[test]
    fn stays_within_the_given_radius() {
        let file = File::open("test/circle.json").unwrap();
        let coordinates: Vec<TestCoordinates> = serde_json::from_reader(file).unwrap();
        let points: Vec<Point<f64>> = coordinates
            .into_iter()
            .map(|t| Point::new(t.0, t.1))
            .collect();
        let exterior = LineString(points);
        let polygon = Polygon::new(exterior, vec![]);

        let center_point = Point::new(9.177789688110352, 48.776781529534965);
        for _ in 0..1000 {
            let random_point = get_random_point(&center_point, 9.9);
            let contained = polygon.contains(&random_point);
            assert_eq!(contained, true);
        }
    }
}

fn handle_rnd(matches: &ArgMatches) {
    let center_loc = matches.values_of("location").unwrap();
    let center_point = parse_point(center_loc).unwrap();
    let rad_str = matches.value_of("radius").unwrap();
    let rad = rad_str.parse::<f64>().unwrap();
    let random_point = get_random_point(&center_point, rad);

    println!("{},{}", random_point.x(), random_point.y());
}

fn handle_loc(matches: &ArgMatches) {
    let place = matches.value_of("place").unwrap();
    match search(place) {
        Ok(data) => println!("{}", data),
        Err(e) => bail_out(e.description()),
    }
}

fn parse_point(mut loc: Values) -> Result<Point<f64>, Box<Error>> {
    let lon = loc.next().ok_or("Argument parse error")?;
    let lat = loc.next().ok_or("Argument parse error")?;
    let lon_f = lon.parse::<f64>()?;
    let lat_f = lat.parse::<f64>()?;
    Ok(Point::new(lon_f, lat_f))
}

fn direction(point: &Point<f64>, bearing: f64, distance: f64) -> Point<f64> {
    let pi = std::f64::consts::PI;
    let deg_to_rad = pi / 180.;
    let rad_to_deg = 180. / pi;
    let center_lng = deg_to_rad * point.x();
    let center_lat = deg_to_rad * point.y();
    let bearing_rad = deg_to_rad * bearing;

    let rad = distance / EARTH_RADIUS_KM;

    let lat = {
            center_lat.sin() * rad.cos() + center_lat.cos() * rad.sin() * bearing_rad.cos()
        }
        .asin();
    let lng = {
            bearing_rad.sin() * rad.sin() * center_lat.cos()
        }
        .atan2(rad.cos() - center_lat.sin() * lat.sin()) + center_lng;

    Point::new(lng * rad_to_deg, lat * rad_to_deg)
}

#[cfg(test)]
mod direction {
    use super::*;

    #[test]
    fn returns_a_new_point() {
        let point_1 = Point::new(9.177789688110352, 48.776781529534965);
        let point_2 = direction(&point_1, 45., 10.);
        assert_eq!(point_2, Point::new(9.274379723017008, 48.840312896632746));
    }
}

fn handle_dis(matches: &ArgMatches) {
    let loc_1 = matches.values_of("location1").unwrap();
    let point_1 = parse_point(loc_1).unwrap();
    let loc_2 = matches.values_of("location2").unwrap();
    let point_2 = parse_point(loc_2).unwrap();
    let dist = point_1.haversine_distance(&point_2);
    println!("{:.0}", dist);
}

fn main() {
    let app = get_cli_app();
    let matches = app.get_matches();

    match matches.subcommand() {
        ("loc", Some(sub_m)) => handle_loc(sub_m),
        ("rnd", Some(sub_m)) => handle_rnd(sub_m),
        ("rev", Some(sub_m)) => handle_rev(sub_m),
        ("dis", Some(sub_m)) => handle_dis(sub_m),
        _ => bail_out(matches.usage()),
    }
}
