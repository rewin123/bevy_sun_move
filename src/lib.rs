pub mod random_stars;

use bevy::prelude::*;
use std::f32::consts::PI;

// Helper constants
pub const DEGREES_TO_RADIANS: f32 = PI / 180.0;
pub const RADIANS_TO_DEGREES: f32 = 180.0 / PI;

pub struct SunMovePlugin;

impl Plugin for SunMovePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, update_sky_center);
    }
}

// Determine latitude and year fraction from day and night fractions of full cycle
#[derive(Component, Debug, Clone)]
pub struct TimedSkyConfig {
    pub planet_tilt_degrees: f32,
    /// Desired duration of daylight in seconds.
    pub day_duration_secs: f32,
    /// Desired duration of nighttime in seconds.
    pub night_duration_secs: f32,
    /// Desired maximum sun height (altitude) in degrees during the day.
    pub max_sun_height_deg: f32,
    /// The entity representing the sun (usually a DirectionalLight).
    pub sun_entity: Entity,
}

impl Default for TimedSkyConfig {
    fn default() -> Self {
        Self {
            planet_tilt_degrees: 23.5, // Earth's tilt
            sun_entity: Entity::PLACEHOLDER,
            day_duration_secs: 15.0,   // Example: 15s day
            night_duration_secs: 15.0, // Example: 15s night (total cycle 30s)
            max_sun_height_deg: 45.0,
        }
    }
}

/// Calculates required latitude and year fraction to achieve a specific day/night
/// duration ratio and maximum sun height (noon altitude) for a given planet tilt.
///
/// Based on standard astronomical formulas relating day length, noon altitude,
/// latitude, and declination.
///
/// Args:
/// - planet_tilt_degrees: The axial tilt of the planet in degrees.
/// - day_duration_secs: The target duration of daylight in seconds.
/// - night_duration_secs: The target duration of nighttime in seconds.
/// - max_sun_height_deg: The target maximum altitude of the sun in degrees.
///
/// Returns:
/// An `Option<(latitude_degrees, year_fraction, calculated_declination_degrees)>`.
/// Returns `None` if the requested parameters are impossible for the given tilt
/// (e.g., max height too high/low for the day length, or required declination
/// exceeds the planet tilt).
#[allow(non_snake_case)]
pub fn calculate_latitude_yearfraction(
    planet_tilt_degrees: f32,
    day_duration_secs: f32,
    night_duration_secs: f32,
    max_sun_height_deg: f32,
) -> Option<(f32, f32, f32)> {
    let total_duration_secs = day_duration_secs + night_duration_secs;
    let tilt_rad = planet_tilt_degrees.abs() * DEGREES_TO_RADIANS;

    if total_duration_secs <= f32::EPSILON || day_duration_secs < 0.0 || night_duration_secs < 0.0 {
        warn!(
            "Invalid timed durations: day={}s, night={}s. Cannot calculate.",
            day_duration_secs, night_duration_secs
        );
        return None;
    }

    if max_sun_height_deg < -0.1 || max_sun_height_deg > 90.0 + 0.1 {
        // Allow slight floating point deviations
        warn!(
            "Max sun height {:.2}° is outside valid range [0°, 90°]. Cannot calculate.",
            max_sun_height_deg
        );
        return None;
    }

    // Handle edge cases: Perpetual Day/Night or 12/12 cycle
    if day_duration_secs < f32::EPSILON && night_duration_secs > f32::EPSILON {
        // Perpetual Night (day_fraction = 0)
        // Requires sun never rises, i.e. max altitude <= 0.
        if max_sun_height_deg > f32::EPSILON {
            warn!(
                "Perpetual night requested but max sun height is {:.2}°. Impossible.",
                max_sun_height_deg
            );
            return None;
        }
        // Max height is 0. This happens at latitudes where sun circles the horizon.
        // This occurs at latitude = 90 - |dec|. For perpetual night at a pole-like lat,
        // we need dec to be -tilt (NH winter) or +tilt (SH winter).
        // Latitude is 90 - tilt. Year fraction is 0.75 (NH) or 0.25 (SH).
        if tilt_rad < f32::EPSILON {
            warn!("Perpetual night with 0 tilt is impossible unless at equator (12/12 cycle).");
            return None; // 0 tilt implies 12/12 cycle everywhere.
        }
        let calculated_latitude_degrees =
            (90.0 - planet_tilt_degrees.abs()).copysign(-planet_tilt_degrees); // Choose pole opposite tilt
        let calculated_declination_degrees = -planet_tilt_degrees.copysign(planet_tilt_degrees); // Winter solstice dec
        let calculated_year_fraction = if planet_tilt_degrees > 0.0 {
            0.75
        } else {
            0.25
        }; // NH Winter or SH Winter
        // info!("Perpetual night calculation: Lat {:.2}°, Dec {:.2}°, YF {:.2}", calculated_latitude_degrees, calculated_declination_degrees, calculated_year_fraction);
        return Some((
            calculated_latitude_degrees,
            calculated_year_fraction,
            calculated_declination_degrees,
        ));
    }

    if night_duration_secs < f32::EPSILON && day_duration_secs > f32::EPSILON {
        // Perpetual Day (day_fraction = 1)
        // Requires sun never sets, i.e. min altitude >= 0.
        // Max height must be > 0 (unless at pole/equinox/tilt=0 which implies 12/12 max height 0).
        if max_sun_height_deg < f32::EPSILON {
            warn!(
                "Perpetual day requested but max sun height is {:.2}°. Impossible (must be > 0 unless 12/12).",
                max_sun_height_deg
            );
            return None; // Perpetual day usually has max height > 0. Max height 0 is the 12/12 case.
        }
        // Max height > 0. Perpetual day happens at latitudes polewards of 90 - tilt during summer solstice.
        // Max height = 90 - |lat - dec|. Min height = 90 - |lat + dec|.
        // At lat = 90 - tilt, summer solstice (dec=tilt), max height = 90 - (90-tilt - tilt) = 2*tilt. Min height = 90 - (90-tilt + tilt) = 0.
        // For max height H > 0 and perpetual day, required dec = H/2, required lat = 90 - H/2.
        if tilt_rad < f32::EPSILON {
            warn!("Perpetual day with 0 tilt is impossible unless at equator (12/12 cycle).");
            return None; // 0 tilt implies 12/12 cycle everywhere.
        }
        let max_height_rad = max_sun_height_deg * DEGREES_TO_RADIANS;
        let required_dec_rad = max_height_rad / 2.0;
        if required_dec_rad.abs() > tilt_rad + f32::EPSILON {
            warn!(
                "Required declination {:.2}° for perpetual day with max height {:.2}° exceeds planet tilt {:.2}°. Impossible.",
                required_dec_rad * RADIANS_TO_DEGREES,
                max_sun_height_deg,
                planet_tilt_degrees
            );
            return None;
        }
        let calculated_latitude_degrees =
            (90.0 * DEGREES_TO_RADIANS - required_dec_rad) * RADIANS_TO_DEGREES;
        let calculated_declination_degrees = required_dec_rad * RADIANS_TO_DEGREES;
        // Summer solstice requires dec > 0 if lat > 0, or dec < 0 if lat < 0.
        // We aim for positive latitude hemisphere:
        let final_lat_deg = calculated_latitude_degrees.copysign(planet_tilt_degrees); // Use tilt sign to pick hemisphere
        let final_dec_deg = calculated_declination_degrees.copysign(planet_tilt_degrees); // Dec must match hemi for summer
        let sin_yf_angle = final_dec_deg * DEGREES_TO_RADIANS / tilt_rad;
        let phi = sin_yf_angle.clamp(-1.0, 1.0).asin();
        let calculated_year_fraction = if final_dec_deg >= 0.0 {
            phi / (2.0 * PI)
        } else {
            0.5 - phi / (2.0 * PI)
        };

        // info!("Perpetual day calculation: Lat {:.2}°, Dec {:.2}°, YF {:.2}", final_lat_deg, final_dec_deg, calculated_year_fraction);
        return Some((final_lat_deg, calculated_year_fraction, final_dec_deg));
    }

    if total_duration_secs <= f32::EPSILON {
        warn!("Total duration is zero.");
        return None;
    }

    let day_fraction = day_duration_secs / total_duration_secs;
    let max_height_rad = max_sun_height_deg * DEGREES_TO_RADIANS;

    let C = (PI * day_fraction).cos();
    let S_h = max_height_rad.sin();

    // Derived relations:
    // cos(lat_rad - dec_rad) = sin(max_height_rad)
    // cos(lat_rad + dec_rad) = sin(max_height_rad) * (1 + cos(PI * day_fraction)) / (1 - cos(PI * day_fraction))

    let term_for_cos_sum = if (1.0 - C).abs() < f32::EPSILON {
        // Handle day_fraction near 0 (C near 1)
        if S_h > f32::EPSILON {
            // Max height > 0 with day fraction near 0 (perpetual night)
            warn!(
                "Impossible combination: Max height {:.2}° requires sun rise, but day fraction {:.2} requests near perpetual night.",
                max_sun_height_deg, day_fraction
            );
            return None;
        } else {
            // Max height near 0 with day fraction near 0 (perpetual night on horizon)
            // This case should be handled by the perpetual night block above.
            // If we reach here, something is slightly off. Return None or default.
            warn!("Reached indeterminate case for cos(lat+dec) near day_fraction 0.");
            return None;
        }
    } else {
        S_h * (1.0 + C) / (1.0 - C)
    };

    if term_for_cos_sum.abs() > 1.0 + f32::EPSILON {
        warn!(
            "Impossible combination: Max height {:.2}° and day fraction {:.2} requires cos(lat+dec) value {:.2} outside [-1, 1].",
            max_sun_height_deg, day_fraction, term_for_cos_sum
        );
        return None;
    }

    let beta = term_for_cos_sum.clamp(-1.0, 1.0).acos(); // angle for lat + dec
    let alpha = PI / 2.0 - max_height_rad; // angle for |lat - dec| (zenith distance at noon)

    // Note: cos(lat-dec) = sin(h) implies |lat-dec| = PI/2 - h for h in [0, PI/2]
    // The sign of (lat-dec) determines if sun culminates South (+ve) or North (-ve) of zenith.
    // cos(lat+dec) = term_for_cos_sum
    // The sign of (lat+dec) determines the average position relative to equator/solstices.

    // We need to solve the system:
    // lat - dec = +/- alpha
    // lat + dec = +/- beta

    // Let's find candidate lat/dec pairs. There are 4 mathematical pairs, but only 1 or 2
    // will have |dec| <= |tilt| and |lat| <= PI/2.
    // Pairs (lat, dec) in radians:
    let candidates = [
        ((alpha + beta) / 2.0, (beta - alpha) / 2.0), // lat-dec = +alpha, lat+dec = +beta
        ((alpha - beta) / 2.0, (-beta - alpha) / 2.0), // lat-dec = +alpha, lat+dec = -beta
        ((-alpha + beta) / 2.0, (beta + alpha) / 2.0), // lat-dec = -alpha, lat+dec = +beta
        ((-alpha - beta) / 2.0, (-beta + alpha) / 2.0), // lat-dec = -alpha, lat+dec = -beta
    ];

    let mut found_lat_rad = None;
    let mut found_dec_rad = None;

    for (lat_candidate, dec_candidate) in candidates.iter() {
        let lat_deg = lat_candidate * RADIANS_TO_DEGREES;
        let dec_deg = dec_candidate * RADIANS_TO_DEGREES;

        // Check if dec is achievable with the planet tilt
        if dec_deg.abs() <= planet_tilt_degrees.abs() + f32::EPSILON {
            // Check if latitude is valid
            if lat_deg.abs() <= 90.0 + f32::EPSILON {
                // Found a valid pair. Check if it matches our preferred sign combo.
                let current_lat_sign = lat_deg.signum();
                let current_dec_sign = dec_deg.signum();

                let signs_match_preference = (day_fraction > 0.5 && current_lat_sign * current_dec_sign >= 0.0) || // Long day: lat and dec same sign
                    (day_fraction < 0.5 && current_lat_sign * current_dec_sign <= 0.0); // Short day: lat and dec opposite sign

                // If it matches preference, pick it immediately and break.
                // If not, keep searching in case there's another valid one that does.
                // If multiple match preference, the first found in the list order is used.
                if signs_match_preference {
                    found_lat_rad = Some(*lat_candidate);
                    found_dec_rad = Some(*dec_candidate);
                    break; // Found preferred solution
                }

                // If no preferred solution found yet, store *any* valid solution
                // (the last one found in the loop order will be kept if no preferred is found)
                if found_lat_rad.is_none() {
                    found_lat_rad = Some(*lat_candidate);
                    found_dec_rad = Some(*dec_candidate);
                }
            }
        }
    }

    match (found_lat_rad, found_dec_rad) {
        (Some(lat_rad), Some(dec_rad)) => {
            let calculated_latitude_degrees = lat_rad * RADIANS_TO_DEGREES;
            let calculated_declination_degrees = dec_rad * RADIANS_TO_DEGREES;

            // Now find the year fraction corresponding to this declination and tilt
            if tilt_rad < f32::EPSILON {
                // Handle 0 tilt separately
                if dec_rad.abs() > f32::EPSILON {
                    warn!(
                        "Calculated non-zero declination {:.2}° but tilt is 0°. Impossible.",
                        calculated_declination_degrees
                    );
                    return None;
                }
                // If dec is 0 and tilt is 0, any year fraction works, but let's pick equinox.
                return Some((
                    calculated_latitude_degrees,
                    0.0,
                    calculated_declination_degrees,
                ));
            }

            let sin_yf_angle = (dec_rad / tilt_rad).clamp(-1.0, 1.0); // Should be <= 1 from checks, but clamp for safety
            let phi = sin_yf_angle.asin(); // phi is in [-PI/2, PI/2]

            // There are two year fractions per declination (unless at solstice)
            // yf1 maps dec >= 0 to [0, 0.25] and dec < 0 to [0.75, 1)
            let yf1 = if dec_rad >= 0.0 {
                phi / (2.0 * PI)
            } else {
                1.0 + phi / (2.0 * PI)
            };
            // yf2 maps dec >= 0 to [0.25, 0.5] and dec < 0 to (0.5, 0.75]
            let yf2 = if dec_rad >= 0.0 {
                0.5 - phi / (2.0 * PI)
            } else {
                0.5 - phi / (2.0 * PI)
            };

            // Let's choose the year fraction that is closer to the 'expected' season for the day length
            // Long day (df > 0.5) suggests summer-like conditions (yf near 0.25 or 0.75 depending on hemi/tilt sign)
            // Short day (df < 0.5) suggests winter-like conditions (yf near 0.75 or 0.25 depending on hemi/tilt sign)
            // Given we aimed for lat/dec signs matching df, dec > 0 implies NH summer/SH winter half year.
            // dec > 0 is yf in (0, 0.5). yf1 is [0, 0.25], yf2 is [0.25, 0.5]. Pick one closest to 0.25?
            // dec < 0 is yf in (0.5, 1). yf1 is [0.75, 1), yf2 is (0.5, 0.75]. Pick one closest to 0.75?

            let target_yf = if dec_rad >= 0.0 { 0.25 } else { 0.75 };
            let calculated_year_fraction = if (target_yf - yf1).abs() < (target_yf - yf2).abs() {
                yf1
            } else {
                yf2
            };
            // Ensure year fraction is in [0, 1) range
            let final_yf = calculated_year_fraction.fract();
            let final_yf = if final_yf < 0.0 {
                final_yf + 1.0
            } else {
                final_yf
            };

            //  info!("Calculated parameters: Latitude {:.2}°, Declination {:.2}°, Year Fraction {:.4}",
            //        calculated_latitude_degrees, calculated_declination_degrees, final_yf);

            Some((
                calculated_latitude_degrees,
                final_yf,
                calculated_declination_degrees,
            ))
        }
        _ => {
            warn!("No valid latitude/declination found for the given constraints.");
            None
        }
    }
}

#[derive(Component, Debug, Clone)]
#[require(Transform, Visibility)]
pub struct SkyCenter {
    pub latitude_degrees: f32,
    pub planet_tilt_degrees: f32,

    /// Fraction of the year (0.0 to 1.0), where 0.0 is Vernal Equinox.
    pub year_fraction: f32,

    /// Duration of a full day/night cycle in seconds.
    pub cycle_duration_secs: f32,

    /// The entity representing the sun (usually a DirectionalLight).
    pub sun: Entity,

    /// Time elapsed within the current cycle (seconds).
    /// Stored here to allow pausing/setting time easily.
    pub current_cycle_time: f32,
}

impl Default for SkyCenter {
    fn default() -> Self {
        Self {
            latitude_degrees: 0.0,
            planet_tilt_degrees: 23.5,
            year_fraction: 0.0,
            cycle_duration_secs: 600.0, // 10 minutes by default
            sun: Entity::PLACEHOLDER,
            current_cycle_time: 0.0,
        }
    }
}

impl SkyCenter {
    pub fn from_timed_config(timed_config: &TimedSkyConfig) -> Option<Self> {
        let calc = calculate_latitude_yearfraction(
            timed_config.planet_tilt_degrees,
            timed_config.day_duration_secs,
            timed_config.night_duration_secs,
            timed_config.max_sun_height_deg,
        );

        if let Some((latitude, year_fraction, _)) = calc {
            Some(Self {
                latitude_degrees: latitude,
                planet_tilt_degrees: timed_config.planet_tilt_degrees,
                year_fraction,
                cycle_duration_secs: timed_config.day_duration_secs
                    + timed_config.night_duration_secs,
                sun: timed_config.sun_entity,
                current_cycle_time: 0.0,
            })
        } else {
            warn!("Failed to calculate latitude/year_fraction/declination for timed sky config.");
            None
        }
    }

    #[allow(dead_code)]
    fn update_from_timed_config(&mut self, timed_config: &TimedSkyConfig) {
        let calc = calculate_latitude_yearfraction(
            timed_config.planet_tilt_degrees,
            timed_config.day_duration_secs,
            timed_config.night_duration_secs,
            timed_config.max_sun_height_deg,
        );

        if let Some((latitude, year_fraction, _)) = calc {
            self.latitude_degrees = latitude;
            self.year_fraction = year_fraction;
            self.cycle_duration_secs =
                timed_config.day_duration_secs + timed_config.night_duration_secs;
            self.sun = timed_config.sun_entity;
        } else {
            warn!("Failed to calculate latitude/year_fraction/declination for timed sky config.");
        }
    }
}

/// Calculates the sun's direction vector in the observer's local coordinate frame (Y up, X east, Z north).
/// This vector points *from* the observer *towards* the sun.
///
/// Based on standard astronomical formulas converting equatorial coordinates (declination, hour angle)
/// to horizontal coordinates (altitude, azimuth).
///
/// Args:
/// - hour_fraction: Fraction of the day (0.0 to 1.0), where 0.0 is midnight, 0.5 is noon.
/// - latitude_rad: Observer's latitude in radians (-PI/2 to PI/2).
/// - axial_tilt_rad: Planet's axial tilt in radians (e.g., 23.5 degrees for Earth).
/// - year_fraction: Fraction of the year (0.0 to 1.0), where 0.0 is Vernal Equinox.
///
/// Returns:
/// A `Vec3` representing the sun's direction relative to the observer.
/// The vector length is arbitrary, usually normalized.
pub fn calculate_sun_direction(
    hour_fraction: f32,
    latitude_rad: f32,
    axial_tilt_rad: f32,
    year_fraction: f32,
) -> Vec3 {
    // Calculate sun's declination based on axial tilt and time of year.
    // Assuming year_fraction 0.0 is Vernal Equinox (dec=0), 0.25 is Summer Solstice (dec=tilt), etc.
    let year_angle_rad = year_fraction * 2.0 * PI;
    let dec_rad = axial_tilt_rad * year_angle_rad.sin();

    // Calculate Local Hour Angle (LHA). This is angle from local meridian (South/North line).
    // hour_fraction 0.0 is midnight, 0.5 is noon. LHA is 0 at noon, PI 12 hours later.
    // hour_angle_rad from midnight = hour_fraction * 2.0 * PI.
    // Local Hour Angle (HA) is angle west of meridian. HA=0 at noon.
    let hour_angle_rad_from_midnight = hour_fraction * 2.0 * PI;
    let local_hour_angle_rad = hour_angle_rad_from_midnight - PI; // Angle from noon meridian, positive West

    // Calculate sun's altitude (elevation above horizon) and components in local frame.
    // Standard formulas for converting equatorial (Dec, HA) to horizontal (Alt, Azi):
    // sin(alt) = sin(lat)sin(dec) + cos(lat)cos(dec)cos(HA)
    // cos(alt)sin(azi) = cos(dec)sin(HA)              (X component in East-Up-North)
    // cos(alt)cos(azi) = cos(lat)sin(dec) - sin(lat)cos(dec)cos(HA) (Z component in East-Up-North)

    // Y (up) component = sin(altitude)
    let sin_alt = latitude_rad.sin() * dec_rad.sin()
        + latitude_rad.cos() * dec_rad.cos() * local_hour_angle_rad.cos();

    // X (east) component = cos(altitude) * sin(azimuth from North towards East)
    // Z (north) component = cos(altitude) * cos(azimuth from North towards East)
    // We can get these components directly without calculating azimuth explicitly:
    let x_east = dec_rad.cos() * local_hour_angle_rad.sin();
    let z_north = latitude_rad.cos() * dec_rad.sin()
        - latitude_rad.sin() * dec_rad.cos() * local_hour_angle_rad.cos();

    // Construct the direction vector in the observer's local Bevy frame (X east, Y up, Z north)
    let sun_direction_local = Vec3::new(
        x_east,  // X: East
        sin_alt, // Y: Up (sin_alt is already calculated)
        z_north, // Z: North
    );

    // Normalize the vector
    sun_direction_local.normalize()
}

fn update_sky_center(
    mut q_sky_center: Query<(&mut Transform, &mut SkyCenter)>,
    mut q_sun: Query<&mut Transform, Without<SkyCenter>>,
    time: Res<Time>,
) {
    for (mut sky_transforms, mut sky_center) in q_sky_center.iter_mut() {
        // Update time
        sky_center.current_cycle_time += time.delta_secs();
        sky_center.current_cycle_time %= sky_center.cycle_duration_secs; // Cycle time loops

        let hour_fraction = sky_center.current_cycle_time / sky_center.cycle_duration_secs;

        let latitude_rad = sky_center.latitude_degrees * DEGREES_TO_RADIANS;
        let tilt_rad = sky_center.planet_tilt_degrees * DEGREES_TO_RADIANS;
        let year_fraction = sky_center.year_fraction;

        sky_transforms.translation = Vec3::ZERO;
        // Sky sphere rotation axis. Useful for attach stars and celestial bodies to the sky sphere.
        let celestial_pole_axis_local = Vec3::new(0.0, latitude_rad.sin(), latitude_rad.cos());

        // Sky sphere rotation
        let rotation_angle_rad = PI - hour_fraction * 2.0 * PI;
        sky_transforms.rotation =
            Quat::from_axis_angle(celestial_pole_axis_local, rotation_angle_rad);

        let sun_direction_local =
            calculate_sun_direction(hour_fraction, latitude_rad, tilt_rad, year_fraction);

        if let Ok(mut sun_transform) = q_sun.get_mut(sky_center.sun) {
            sun_transform.translation = sun_direction_local;
            sun_transform.look_at(Vec3::ZERO, Vec3::Y); // Ensure the light points towards the origin
        }
    }
}
