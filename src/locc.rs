extern crate reqwest;
extern crate rand;
extern crate serde_json;

use std::error::Error;
use std::boxed::Box;
use clap::{ArgMatches, Values};
use std::fmt;
use std::fmt::Display;
use reqwest::header::UserAgent;
use geo::Point;
use geo::haversine_distance::HaversineDistance;
use rand::distributions::{IndependentSample, Range};
use num_traits::Float;

static NOMINATIM_ENDPOINT: &str = "http://nominatim.openstreetmap.org";
const VERSION: &str = env!("CARGO_PKG_VERSION");
const NAME: &str = env!("CARGO_PKG_NAME");
const EARTH_RADIUS_KM: f64 = 6373.;

#[derive(Deserialize, Debug)]
struct Location {
    lat: String,
    lon: String,
}

impl Location {
    fn to_point(&self) -> Result<Point<f64>, Box<Error>> {
        to_point((self.lon.as_str(), self.lat.as_str()))
    }
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

fn get_random_point(center: &Point<f64>, radius: f64) -> Point<f64> {
    let mut rng = rand::thread_rng();
    let dist_range = Range::new(0., 1.0);
    let rnd_factor = dist_range.ind_sample(&mut rng).sqrt();
    let distance = rnd_factor * radius;
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

fn direction(point: &Point<f64>, bearing: f64, distance: f64) -> Point<f64> {
    let center_lng = point.x().to_radians();
    let center_lat = point.y().to_radians();
    let bearing_rad = bearing.to_radians();

    let rad = distance / EARTH_RADIUS_KM;

    let lat = {
            center_lat.sin() * rad.cos() + center_lat.cos() * rad.sin() * bearing_rad.cos()
        }
        .asin();
    let lng = {
            bearing_rad.sin() * rad.sin() * center_lat.cos()
        }
        .atan2(rad.cos() - center_lat.sin() * lat.sin()) + center_lng;

    Point::new(lng.to_degrees(), lat.to_degrees())
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

fn get_arg_value<'a>(matches: &'a ArgMatches, key: &str) -> Result<&'a str, Box<Error>> {
    matches
        .value_of(key)
        .ok_or(From::from(format!("Could not parse {} argument", key)))
}

fn get_arg_values<'a>(matches: &'a ArgMatches, key: &str) -> Result<Values<'a>, Box<Error>> {
    let values = matches
        .values_of(key)
        .ok_or(format!("Could not parse {} argument", key))?;
    Ok(values)
}

fn parse_loc_string(mut loc: Values) -> Result<(&str, &str), Box<Error>> {
    let lon = loc.next().ok_or("Could not parse longitude")?;
    let lat = loc.next().ok_or("Could not parse latitute")?;
    Ok((lon, lat))
}

fn to_point(loc: (&str, &str)) -> Result<Point<f64>, Box<Error>> {
    let (lon, lat) = loc;
    let lon_f = lon.parse::<f64>()?;
    let lat_f = lat.parse::<f64>()?;
    Ok(Point::new(lon_f, lat_f))
}

fn parse_point(matches: &ArgMatches, key: &str) -> Result<Point<f64>, Box<Error>> {
    get_arg_values(matches, key)
        .and_then(parse_loc_string)
        .and_then(to_point)
}

fn parse_float(matches: &ArgMatches, key: &str) -> Result<f64, Box<Error>> {
    let rad_str = get_arg_value(matches, key)?;
    let rad = rad_str.parse::<f64>()?;
    Ok(rad)
}

pub fn handle_rev(matches: &ArgMatches) -> Result<String, Box<Error>> {
    let (lon, lat) = get_arg_values(matches, "location")
        .and_then(parse_loc_string)?;
    reverse_geocode(lon, lat).map(|place| format!("{}", place))
}

pub fn handle_dis(matches: &ArgMatches) -> Result<String, Box<Error>> {
    let point_1 = parse_point(matches, "location1")?;
    let point_2 = parse_point(matches, "location2")?;
    let dist = point_1.haversine_distance(&point_2);
    Ok(format!("{:.0}", dist))
}

pub fn handle_rnd(matches: &ArgMatches) -> Result<String, Box<Error>> {
    let center_point = match matches.value_of("place") {
        Some(place) => search(place).and_then(|loc| loc.to_point()),
        None => parse_point(matches, "location"),
    }?;
    let rad = parse_float(matches, "radius")?;
    let random_point = get_random_point(&center_point, rad);
    Ok(format!("{},{}", random_point.x(), random_point.y()))
}

pub fn handle_loc(matches: &ArgMatches) -> Result<String, Box<Error>> {
    let place = get_arg_value(matches, "place")?;
    search(place).map(|result| format!("{}", result))
}
