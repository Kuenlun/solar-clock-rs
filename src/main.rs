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

use chrono::{DateTime, Duration, FixedOffset, TimeZone, Utc};
use ndarray::Array1;
use scirs2_interpolate::{MonotonicInterpolator, MonotonicMethod};

mod spa;

// --- Configuration ---

// Solar Reference Timezone: UTC+1 (Fixed, no DST)
const SOLAR_TIMEZONE_OFFSET: i32 = 3600; // seconds

// Target Solar Events (Hour in Solar Reference Timezone)
const TARGET_SUNRISE_HOUR: u32 = 8;
const TARGET_TRANSIT_HOUR: u32 = 14;
const TARGET_SUNSET_HOUR: u32 = 20;

#[derive(Debug, Clone, Copy)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

fn main() {
    // 1. Normalize Input (Current Date in UTC)
    let dt_input = Utc::now();
    // Test coordinates (Madrid, Spain)
    let lat = 40.4168;
    let lon = -3.7038;

    println!("=== Solar Clock Calculation ===");
    println!("Input Time (UTC): {}", dt_input);
    println!("Coordinates: Lat {}, Lon {}", lat, lon);

    // 2. Calculate Reference Points (UTC) & 3. Define Target Points
    // We analyze Yesterday, Today, and Tomorrow to ensure we have points covering the input time
    let points = build_interpolation_model(dt_input, lat, lon);

    if points.len() < 2 {
        println!("Insufficient solar data to calculate solar clock (Polar region?).");
        return;
    }

    // 4. Build Interpolation Model (PCHIP)
    // Points are (x=Real_UTC_Timestamp, y=Delta_Seconds)
    // Split into X and Y vectors for the library
    let x_vals: Vec<f64> = points.iter().map(|p| p.x).collect();
    let y_vals: Vec<f64> = points.iter().map(|p| p.y).collect();

    let x_arr = Array1::from(x_vals);
    let y_arr = Array1::from(y_vals);

    let input_timestamp =
        dt_input.timestamp() as f64 + (dt_input.timestamp_subsec_nanos() as f64 / 1_000_000_000.0);

    // Attempt interpolation
    // Arg 4: extrapolate? assume false
    if let Ok(interpolator) =
        MonotonicInterpolator::new(&x_arr.view(), &y_arr.view(), MonotonicMethod::Pchip, false)
    {
        // Trait method usually is 'interpolate' or 'eval'
        let delta_seconds = interpolator.evaluate(input_timestamp).unwrap_or(0.0);

        // 5. Calculate Output
        // Solar time timestamp = Input + Delta

        let extra_seconds = delta_seconds as i64;
        let extra_nanos = ((delta_seconds - extra_seconds as f64) * 1_000_000_000.0) as u32;

        let dt_solar_utc = dt_input
            .checked_add_signed(Duration::seconds(extra_seconds))
            .and_then(|d| d.checked_add_signed(Duration::nanoseconds(extra_nanos as i64)))
            .unwrap_or(dt_input);

        let solar_tz = FixedOffset::east_opt(SOLAR_TIMEZONE_OFFSET).unwrap();
        let dt_solar_final = dt_solar_utc.with_timezone(&solar_tz);

        println!("\n--- Result ---");
        println!("Input Time (UTC): {}", dt_input);
        println!("Delta (Model): {:.3} seconds", delta_seconds);
        println!(
            "Solar Clock Time: {}",
            dt_solar_final.format("%Y-%m-%d %H:%M:%S %z")
        );
        println!(
            "(Target: Sunrise {:02}:00, Noon {:02}:00, Sunset {:02}:00)",
            TARGET_SUNRISE_HOUR, TARGET_TRANSIT_HOUR, TARGET_SUNSET_HOUR
        );
    } else {
        println!("Failed to create PCHIP interpolator.");
    }
}

fn build_interpolation_model(center_date: DateTime<Utc>, lat: f64, lon: f64) -> Vec<Point> {
    let mut points = Vec::new();

    // Iterate -1, 0, +1 days
    for day_offset in -1..=1 {
        let date_eval = center_date + Duration::days(day_offset);
        let solar_data = spa::calculate_solar_data(date_eval, lat, lon);

        // Solar Timezone for Targets
        let solar_tz = FixedOffset::east_opt(SOLAR_TIMEZONE_OFFSET).unwrap();

        // Helper to add point if event exists
        // Event Type 1: Sunrise
        if let Some(real_sunrise) = solar_data.sunrise {
            let target_sunrise = get_target_time(date_eval, TARGET_SUNRISE_HOUR, solar_tz);
            add_point(&mut points, real_sunrise, target_sunrise);
        }

        // Event Type 2: Transit (Noon)
        // Transit always exists in spa return (unless error, but strictly type is DateTime)
        let real_transit = solar_data.transit;
        let target_transit = get_target_time(date_eval, TARGET_TRANSIT_HOUR, solar_tz);
        add_point(&mut points, real_transit, target_transit);

        // Event Type 3: Sunset
        if let Some(real_sunset) = solar_data.sunset {
            let target_sunset = get_target_time(date_eval, TARGET_SUNSET_HOUR, solar_tz);
            add_point(&mut points, real_sunset, target_sunset);
        }
    }

    // Sort points by x (time) as required for interpolation
    points.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap());

    // Dedup? Usually distinct events.
    points
}

fn get_target_time(base_date: DateTime<Utc>, hour: u32, tz: FixedOffset) -> DateTime<Utc> {
    // Construct the target time in the Fixed Timezone
    // converting base_date to FixedTZ to get Y-M-D
    let local_date = base_date.with_timezone(&tz).date_naive();

    // Construct DateTime at that Y-M-D with specific hour
    let target_naive = local_date.and_hms_opt(hour, 0, 0).unwrap();

    // Convert back to DateTime<FixedOffset> then to UTC
    let target_fixed = tz.from_local_datetime(&target_naive).unwrap();

    target_fixed.with_timezone(&Utc)
}

fn add_point(points: &mut Vec<Point>, real_time: DateTime<Utc>, target_time: DateTime<Utc>) {
    let x = real_time.timestamp() as f64 + (real_time.timestamp_subsec_nanos() as f64 / 1e9);

    // Y = Target (UTC) - Real (UTC)
    let target_ts =
        target_time.timestamp() as f64 + (target_time.timestamp_subsec_nanos() as f64 / 1e9);
    let y = target_ts - x;

    points.push(Point { x, y });
}
