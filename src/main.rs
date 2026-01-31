use chrono::Utc;
use spa::{StdFloatOps, SunriseAndSet, sunrise_and_set};

fn main() {
    // Use the current date
    let dt = Utc::now();

    // Test coordinates (Madrid, Spain)
    let lat = 40.4168;
    let lon = -3.7038;

    println!(
        "Calculating solar data for coordinates ({}, {}) on date {}",
        lat,
        lon,
        dt.format("%Y-%m-%d")
    );

    // The calculation of 'noon' (solar noon) is not direct with this library,
    // but we can show sunrise and sunset.
    match sunrise_and_set::<StdFloatOps>(dt, lat, lon) {
        Ok(SunriseAndSet::Daylight(sunrise, sunset)) => {
            println!("Sunrise: {}", sunrise);
            println!("Sunset: {}", sunset);

            // Simple estimation of solar noon (midpoint)
            let duration = sunset.signed_duration_since(sunrise);
            let noon = sunrise + (duration / 2);
            println!("Solar noon: {}", noon);
        }
        Ok(SunriseAndSet::PolarDay) => println!("Polar day (the sun does not set)"),
        Ok(SunriseAndSet::PolarNight) => println!("Polar night (the sun does not rise)"),
        Err(e) => eprintln!("Error calculating: {:?}", e),
    }
}
