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
