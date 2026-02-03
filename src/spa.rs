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

use chrono::{DateTime, Datelike, Duration, Utc};
use std::f64::consts::PI;

pub struct SolarData {
    pub sunrise: Option<DateTime<Utc>>,
    pub sunset: Option<DateTime<Utc>>,
    pub transit: DateTime<Utc>,
}

/// Calculates the astronomical sunrise, sunset and sun transit times in UTC.
/// Ported from pysolar (http://pysolar.org/).
pub fn calculate_solar_data(date: DateTime<Utc>, lat: f64, lon: f64) -> SolarData {
    let day = date.ordinal() as f64;

    // 1. Calculate Declination
    // TT = 2 * math.pi * day / 366
    let tt_decl = 2.0 * PI * day / 366.0;

    // solar declination in degrees
    let decl_deg = 0.322003
        - 22.971 * tt_decl.cos()
        - 0.357898 * (2.0 * tt_decl).cos()
        - 0.14398 * (3.0 * tt_decl).cos()
        + 3.94638 * tt_decl.sin()
        + 0.019334 * (2.0 * tt_decl).sin()
        + 0.05928 * (3.0 * tt_decl).sin();

    let decl_rad = decl_deg.to_radians();

    // 2. Calculate Time Adjustment Angle
    // TT = math.radians(279.134 + 0.985647 * day)
    let tt_time = (279.134 + 0.985647 * day).to_radians();

    // Time adjustment in hours (Equation of Time component)
    let time_adst_hours = (5.0323 - 100.976 * tt_time.sin()
        + 595.275 * (2.0 * tt_time).sin()
        + 3.6858 * (3.0 * tt_time).sin()
        - 12.47 * (4.0 * tt_time).sin()
        - 430.847 * tt_time.cos()
        + 12.5024 * (2.0 * tt_time).cos()
        + 18.25 * (3.0 * tt_time).cos())
        / 3600.0;

    // 3. Time of Noon (TON) in hours from midnight
    // TON = 12 + SHA / 15.0 - time_adst
    // For UTC: SHA = -longitude_deg
    let ton_hours = 12.0 - lon / 15.0 - time_adst_hours;

    let midnight = date.date_naive().and_hms_opt(0, 0, 0).unwrap().and_utc();
    let transit = midnight + Duration::microseconds((ton_hours * 3_600_000_000.0) as i64);

    // 4. Hour Angle (ha) calculation for Sunrise/Sunset
    // cos(ha) = (cos(90.833) / (cos(lat) * cos(decl))) - tan(lat)*tan(decl)
    let lat_rad = lat.to_radians();
    let zenith_rad = 90.833f64.to_radians();

    let cos_ha =
        (zenith_rad.cos() / (lat_rad.cos() * decl_rad.cos())) - (lat_rad.tan() * decl_rad.tan());

    let (sunrise, sunset) = if cos_ha.abs() <= 1.0 {
        let ha_rad = cos_ha.acos();
        let ha_hours = ha_rad * (12.0 / PI); // Convert radians to hours

        let sunrise_hours = ton_hours - ha_hours;
        let sunset_hours = ton_hours + ha_hours;

        (
            Some(midnight + Duration::microseconds((sunrise_hours * 3_600_000_000.0) as i64)),
            Some(midnight + Duration::microseconds((sunset_hours * 3_600_000_000.0) as i64)),
        )
    } else {
        (None, None)
    };

    SolarData {
        sunrise,
        sunset,
        transit,
    }
}
