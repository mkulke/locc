locc
===

CLI for interacting with location services, written in Rust. Nominatim Usage Policy (https://operations.osmfoundation.org/policies/nominatim) applies.

```
cargo build
cargo build --release
./target/release/locc --help
Location utility 0.5.0
Magnus Kulke <mkulke@gmail.com>

USAGE:
    locc [SUBCOMMAND]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

SUBCOMMANDS:
    dis     Return the distance between two points in meters
    help    Prints this message or the help of the given subcommand(s)
    loc     Retrieve a location via search string
    p2g     Convert polyline to geojson
    rev     Reverse geocode a place via location
    rnd     Get a random location within a km radius
```
