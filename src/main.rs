/*!
solar-clock-rs - High-precision solar clock calculator
Copyright (C) 2026  Juan Luis Leal Contreras (Kuenlun)

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU General Public License for more details.

You should have received a copy of the GNU General Public License
along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/

use chrono::{Duration, Utc};
use spa::{StdFloatOps, SunriseAndSet, solar_position, sunrise_and_set};

fn find_true_solar_noon(
    approx_noon: chrono::DateTime<Utc>,
    lat: f64,
    lon: f64,
) -> chrono::DateTime<Utc> {
    // Define objective function: given an offset in nanoseconds from approx_noon, returns the zenith
    let get_zenith = |offset_nanos: i64| -> f64 {
        let t = approx_noon + Duration::nanoseconds(offset_nanos);
        match solar_position::<StdFloatOps>(t, lat, lon) {
            Ok(pos) => pos.zenith_angle,
            Err(_) => f64::MAX, // If it fails, return infinity to discard it
        }
    };

    // Golden Section Search to find the minimum
    let phi = (1.0 + 5.0_f64.sqrt()) / 2.0;
    let resphi = 2.0 - phi;

    // Search window: +/- 20 minutes in nanoseconds
    let window_nanos = 20 * 60 * 1_000_000_000i64;

    let mut a = -window_nanos;
    let mut b = window_nanos;
    let mut c = a + (resphi * (b as f64 - a as f64)) as i64;
    let mut d = b - (resphi * (b as f64 - a as f64)) as i64;

    // Evaluate the function at points c and d
    let mut fc = get_zenith(c);
    let mut fd = get_zenith(d);

    // Iterate until we have microsecond precision (or close enough)
    // 1000 iterations are enough for nanosecond precision, but we stop
    // when the window is small
    while (b - a).abs() > 1000 {
        // 1 microsecond precision (1000 ns)
        if fc < fd {
            b = d;
            d = c;
            fd = fc;
            c = a + (resphi * (b as f64 - a as f64)) as i64;
            fc = get_zenith(c);
        } else {
            a = c;
            c = d;
            fc = fd;
            d = b - (resphi * (b as f64 - a as f64)) as i64;
            fd = get_zenith(d);
        }
    }

    let optimal_offset = (a + b) / 2;
    approx_noon + Duration::nanoseconds(optimal_offset)
}

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
            let approx_noon = sunrise + (duration / 2);
            println!("Solar noon (approx): {}", approx_noon);

            // Refined calculation (Minimum Zenith search)
            let true_noon = find_true_solar_noon(approx_noon, lat, lon);
            println!("Solar noon (exact):  {}", true_noon);
        }
        Ok(SunriseAndSet::PolarDay) => println!("Polar day (the sun does not set)"),
        Ok(SunriseAndSet::PolarNight) => println!("Polar night (the sun does not rise)"),
        Err(e) => eprintln!("Error calculating: {:?}", e),
    }
}
