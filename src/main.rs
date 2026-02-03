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

use chrono::{DateTime, Duration, FixedOffset, Local, NaiveTime, TimeZone, Utc};
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

// Coordinates
const LOCAL_COORDINATES: Coordinates = Coordinates {
    latitude: 38.34599467937726,
    longitude: -0.49068757240971655,
};

#[derive(Debug, Clone, Copy)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Copy)]
pub struct Coordinates {
    pub latitude: f64,
    pub longitude: f64,
}

#[derive(Debug, Clone, Copy)]
pub struct SolarTargets {
    pub sunrise: NaiveTime,
    pub transit: NaiveTime,
    pub sunset: NaiveTime,
}

fn main() {
    // Test coordinates
    let targets = SolarTargets {
        sunrise: NaiveTime::from_hms_opt(TARGET_SUNRISE_HOUR, 0, 0).unwrap(),
        transit: NaiveTime::from_hms_opt(TARGET_TRANSIT_HOUR, 0, 0).unwrap(),
        sunset: NaiveTime::from_hms_opt(TARGET_SUNSET_HOUR, 0, 0).unwrap(),
    };

    println!("=== Solar Clock Calculation (Multi-Input Local) ===");
    println!(
        "Coordinates: Lat {}, Lon {}",
        LOCAL_COORDINATES.latitude, LOCAL_COORDINATES.longitude
    );

    // Define Inputs in LOCAL time (system time)
    // 1. Current Date in Local
    let now = Local::now();
    let inputs = vec![now];

    for dt_input in inputs {
        process_solar_clock(dt_input, LOCAL_COORDINATES, targets);
    }
}

fn process_solar_clock(dt_input: DateTime<Local>, coordinates: Coordinates, targets: SolarTargets) {
    if let Some((delta_seconds, dt_solar_final)) =
        calculate_solar_clock(dt_input, coordinates, targets)
    {
        println!("\nInput Time (Local): {}", dt_input); // Show Local
        println!("Delta (Model):      {:.3} seconds", delta_seconds);
        println!(
            "Solar Clock Time:   {}",
            dt_solar_final.format("%Y-%m-%d %H:%M:%S %z")
        );
    } else {
        println!("Insufficient solar data or interpolation failed.");
    }
}

fn calculate_solar_clock(
    dt_input: DateTime<Local>,
    coordinates: Coordinates,
    targets: SolarTargets,
) -> Option<(f64, DateTime<FixedOffset>)> {
    // Convert Local Input to UTC for calculation
    let dt_input_utc = dt_input.with_timezone(&Utc);

    // 2. Calculate Reference Points (UTC) & 3. Define Target Points
    // We analyze Yesterday, Today, and Tomorrow to ensure we have points covering the input time
    let points = build_interpolation_model(dt_input_utc, coordinates, targets);

    if points.len() < 2 {
        // println!("Insufficient solar data to calculate solar clock (Polar region?).");
        return None;
    }

    // 4. Build Interpolation Model (PCHIP)
    // Points are (x=Real_UTC_Timestamp, y=Delta_Seconds)
    // Split into X and Y vectors for the library
    let x_vals: Vec<f64> = points.iter().map(|p| p.x).collect();
    let y_vals: Vec<f64> = points.iter().map(|p| p.y).collect();

    let x_arr = Array1::from(x_vals);
    let y_arr = Array1::from(y_vals);

    let input_timestamp = dt_input_utc.timestamp() as f64
        + (dt_input_utc.timestamp_subsec_nanos() as f64 / 1_000_000_000.0);

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

        let dt_solar_utc = dt_input_utc
            .checked_add_signed(Duration::seconds(extra_seconds))
            .and_then(|d| d.checked_add_signed(Duration::nanoseconds(extra_nanos as i64)))
            .unwrap_or(dt_input_utc);

        let solar_tz = FixedOffset::east_opt(SOLAR_TIMEZONE_OFFSET).unwrap();
        let dt_solar_final = dt_solar_utc.with_timezone(&solar_tz);

        return Some((delta_seconds, dt_solar_final));
    }

    None
}

fn build_interpolation_model(
    center_date: DateTime<Utc>,
    coordinates: Coordinates,
    targets: SolarTargets,
) -> Vec<Point> {
    let mut points = Vec::new();

    // Iterate -1, 0, +1 days
    for day_offset in -1..=1 {
        let date_eval = center_date + Duration::days(day_offset);
        let solar_data =
            spa::calculate_solar_data(date_eval, coordinates.latitude, coordinates.longitude);

        // Solar Timezone for Targets
        let solar_tz = FixedOffset::east_opt(SOLAR_TIMEZONE_OFFSET).unwrap();

        // Helper to add point if event exists
        // Event Type 1: Sunrise
        if let Some(real_sunrise) = solar_data.sunrise {
            let target_sunrise = get_target_time(real_sunrise, targets.sunrise, solar_tz);
            add_point(&mut points, real_sunrise, target_sunrise);
        }

        // Event Type 2: Transit (Noon)
        // Transit always exists in spa return (unless error, but strictly type is DateTime)
        let real_transit = solar_data.transit;
        let target_transit = get_target_time(real_transit, targets.transit, solar_tz);
        add_point(&mut points, real_transit, target_transit);

        // Event Type 3: Sunset
        if let Some(real_sunset) = solar_data.sunset {
            let target_sunset = get_target_time(real_sunset, targets.sunset, solar_tz);
            add_point(&mut points, real_sunset, target_sunset);
        }
    }

    // Sort points by x (time) as required for interpolation
    points.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap());

    // Dedup? Usually distinct events.
    points
}

fn get_target_time(
    base_date: DateTime<Utc>,
    target_time: NaiveTime,
    tz: FixedOffset,
) -> DateTime<Utc> {
    // Construct the target time in the Fixed Timezone
    // converting base_date to FixedTZ to get Y-M-D
    let local_date = base_date.with_timezone(&tz).date_naive();

    // Construct DateTime at that Y-M-D with specific hour
    let target_naive = local_date.and_time(target_time);

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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn assert_near_time(dt: DateTime<FixedOffset>, expected_hms: &str) {
        let expected_str = dt.format("%Y-%m-%d").to_string() + " " + expected_hms + " +0100";
        let expected = DateTime::parse_from_str(&expected_str, "%Y-%m-%d %H:%M:%S %z")
            .expect("Failed to parse expected time");

        let diff = dt.signed_duration_since(expected).num_seconds().abs();
        assert!(
            diff <= 2,
            "Time {} is too far from expected {} (diff {}s)",
            dt,
            expected_hms,
            diff
        );
    }

    #[test]
    /// Verifies the solar clock algorithm using hardcoded reference data from the project's start date (2026-02-03).
    /// Ensures stability of the calculation against known correct values.
    fn test_solar_clock_algorithm_with_fixed_reference_date() {
        let coordinates = Coordinates {
            latitude: 38.34599467937726,
            longitude: -0.49068757240971655,
        };

        // ORIGINAL fixed targets for this test case
        let test_targets = SolarTargets {
            sunrise: NaiveTime::from_hms_opt(8, 0, 0).unwrap(),
            transit: NaiveTime::from_hms_opt(14, 0, 0).unwrap(),
            sunset: NaiveTime::from_hms_opt(20, 0, 0).unwrap(),
        };

        // Times provided (UTC+1)
        // Note: These inputs are truncated to seconds (from user output),
        // so the resulting Solar Time might not be EXACTLY XX:00:00 due to missing milliseconds in input.
        // We use a small tolerance (e.g., +/- 2 seconds).

        let tz = FixedOffset::east_opt(3600).unwrap();

        let sunrise_input = tz
            .with_ymd_and_hms(2026, 2, 3, 8, 6, 6)
            .unwrap()
            .with_timezone(&Local);
        let transit_input = tz
            .with_ymd_and_hms(2026, 2, 3, 13, 15, 43)
            .unwrap()
            .with_timezone(&Local);
        let sunset_input = tz
            .with_ymd_and_hms(2026, 2, 3, 18, 25, 19)
            .unwrap()
            .with_timezone(&Local);

        let (_, solar_sunrise) = calculate_solar_clock(sunrise_input, coordinates, test_targets)
            .expect("Sunrise calc failed");
        let (_, solar_transit) = calculate_solar_clock(transit_input, coordinates, test_targets)
            .expect("Transit calc failed");
        let (_, solar_sunset) = calculate_solar_clock(sunset_input, coordinates, test_targets)
            .expect("Sunset calc failed");

        // Use hardcoded assertions as requested for this specific test case
        assert_near_time(solar_sunrise, "08:00:00");
        assert_near_time(solar_transit, "14:00:00");
        assert_near_time(solar_sunset, "20:00:00");
    }

    #[test]
    /// Verifies that the solar clock targets (08:00, 14:00, 20:00) correspond correctly
    /// to the real astronomical events (Sunrise, Transit, Sunset) for the current execution date.
    fn test_solar_clock_targets_for_current_date() {
        let coordinates = LOCAL_COORDINATES;

        // Use GLOBAL targets for this test
        let test_targets = SolarTargets {
            sunrise: NaiveTime::from_hms_opt(TARGET_SUNRISE_HOUR, 0, 0).unwrap(),
            transit: NaiveTime::from_hms_opt(TARGET_TRANSIT_HOUR, 0, 0).unwrap(),
            sunset: NaiveTime::from_hms_opt(TARGET_SUNSET_HOUR, 0, 0).unwrap(),
        };

        // Use today's date
        let now = Local::now();
        // Construct a noon time for 'today' to query SPA
        let target_date_utc = now
            .date_naive()
            .and_hms_opt(12, 0, 0)
            .unwrap()
            .and_local_timezone(Local)
            .unwrap()
            .with_timezone(&Utc);

        let solar_data =
            spa::calculate_solar_data(target_date_utc, coordinates.latitude, coordinates.longitude);

        // Expected targets from global config
        let target_sunrise_time = format!("{:02}:00:00", TARGET_SUNRISE_HOUR);
        let target_transit_time = format!("{:02}:00:00", TARGET_TRANSIT_HOUR);
        let target_sunset_time = format!("{:02}:00:00", TARGET_SUNSET_HOUR);

        // Sunrise
        if let Some(sunrise_utc) = solar_data.sunrise {
            let sunrise_local = sunrise_utc.with_timezone(&Local);
            let (_, solar_sunrise) =
                calculate_solar_clock(sunrise_local, coordinates, test_targets)
                    .expect("Sunrise calc failed");
            assert_near_time(solar_sunrise, &target_sunrise_time);
        } else {
            println!("Skipping sunrise check (polar night/day)");
        }

        // Transit
        let transit_local = solar_data.transit.with_timezone(&Local);
        let (_, solar_transit) = calculate_solar_clock(transit_local, coordinates, test_targets)
            .expect("Transit calc failed");
        assert_near_time(solar_transit, &target_transit_time);

        // Sunset
        if let Some(sunset_utc) = solar_data.sunset {
            let sunset_local = sunset_utc.with_timezone(&Local);
            let (_, solar_sunset) = calculate_solar_clock(sunset_local, coordinates, test_targets)
                .expect("Sunset calc failed");
            assert_near_time(solar_sunset, &target_sunset_time);
        } else {
            println!("Skipping sunset check (polar night/day)");
        }
    }

    #[test]
    /// Verifies that the solar clock calculation is continuous effectively handling
    /// Civil Time discontinuities (like Daylight Saving Time).
    /// Uses the Spring Forward transition (02:00 -> 03:00) where the clock jumps.
    /// We verify that the Solar Time 02:00+01:00 (pre-jump) and 03:00+02:00 (post-jump)
    /// (which represent the same UTC instant or continuous instants) produce consistent solar times.
    fn test_solar_clock_continuity_across_dst_change() {
        let coordinates = LOCAL_COORDINATES;
        let targets = SolarTargets {
            sunrise: NaiveTime::from_hms_opt(TARGET_SUNRISE_HOUR, 0, 0).unwrap(),
            transit: NaiveTime::from_hms_opt(TARGET_TRANSIT_HOUR, 0, 0).unwrap(),
            sunset: NaiveTime::from_hms_opt(TARGET_SUNSET_HOUR, 0, 0).unwrap(),
        };

        // Construct inputs representing a DST transition (e.g., Europe late March)
        // Instant A: 02:00:00 +01:00 (Civil time right before/at jump) -> 01:00:00 UTC
        // Instant B: 03:00:00 +02:00 (Civil time right after jump)     -> 01:00:00 UTC
        // These represent the same physical moment. The solar clock should yield the EXACT same result.

        let offset_cet = FixedOffset::east_opt(3600).unwrap(); // UTC+1
        let offset_cest = FixedOffset::east_opt(7200).unwrap(); // UTC+2

        let dt_a = offset_cet
            .with_ymd_and_hms(2026, 3, 29, 2, 0, 0)
            .unwrap()
            .with_timezone(&Local);
        let dt_b = offset_cest
            .with_ymd_and_hms(2026, 3, 29, 3, 0, 0)
            .unwrap()
            .with_timezone(&Local);

        let (_, solar_a) =
            calculate_solar_clock(dt_a, coordinates, targets).expect("Calc A failed");
        let (_, solar_b) =
            calculate_solar_clock(dt_b, coordinates, targets).expect("Calc B failed");

        assert_eq!(
            solar_a, solar_b,
            "Solar clock should be identical for the same UTC instant despite civil time jump"
        );
    }
}
