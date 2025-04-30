pub mod random_stars;


use bevy::{
    pbr::{light_consts::lux, AmbientLight, DirectionalLight}, prelude::*
};
use std::f32::consts::PI;


// Helper constants
pub const DEGREES_TO_RADIANS: f32 = PI / 180.0;
pub const RADIANS_TO_DEGREES: f32 = 180.0 / PI;

pub struct SunMovePlugin;

impl Plugin for SunMovePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, update_sky_center);
        app.add_systems(Update, setup_timed_sky_center);
    }
}

// Determine latitude and year fraction from day and night fractions of full cycle
#[derive(Component, Debug, Clone)]
pub struct TimedSkyCenter {
    pub planet_tilt_degrees: f32,
    pub sun: Entity,
    /// Desired duration of daylight in seconds.
    pub day_duration_secs: f32,
    /// Desired duration of nighttime in seconds.
    pub night_duration_secs: f32,
}

impl Default for TimedSkyCenter {
    fn default() -> Self {
        Self {
            planet_tilt_degrees: 23.5, // Earth's tilt
            sun: Entity::PLACEHOLDER,
            day_duration_secs: 15.0, // Example: 15s day
            night_duration_secs: 15.0, // Example: 15s night (total cycle 30s)
        }
    }
}

fn setup_timed_sky_center(
    mut commands: Commands,
    mut q_timed_sky_center: Query<(Entity, &TimedSkyCenter, Option<&mut SkyCenter>), Changed<TimedSkyCenter>>,
) {
    for (entity, timed_config, mut sky_center_option) in q_timed_sky_center.iter_mut() {
        let total_duration_secs = timed_config.day_duration_secs + timed_config.night_duration_secs;
        let tilt_rad = timed_config.planet_tilt_degrees * DEGREES_TO_RADIANS;

        if total_duration_secs <= 0.0 {
             warn!("TimedSkyCenter on entity {:?} has invalid total duration ({} day + {} night). Defaulting to 12/12 cycle at equator.",
                   entity, timed_config.day_duration_secs, timed_config.night_duration_secs);

             // Default to standard 12h/12h cycle at equator
             let sky_center = SkyCenter {
                 latitude_degrees: 0.0,
                 planet_tilt_degrees: timed_config.planet_tilt_degrees, // Keep the configured tilt
                 year_fraction: 0.0, // Equinox
                 cycle_duration_secs: 600.0, // Default 10 min cycle
                 sun: timed_config.sun,
                 current_cycle_time: 0.0, // Start at midnight
             };

             commands.entity(entity)
                 .insert(sky_center);

             continue; // Move to next entity
        }

        let day_fraction = timed_config.day_duration_secs / total_duration_secs;

        // The magnitude of the Hour Angle (angle from meridian) at sunrise/sunset
        // is PI * (day_fraction).
        // Ref: cos(HA) = -tan(latitude) * tan(declination)
        // We choose Summer Solstice (year_fraction = 0.25) for simplicity,
        // where Declination = Planet Tilt.
        let required_cos_ha = (PI * day_fraction).cos(); // cos(Hour Angle at sunset/sunrise)
        let declination_rad = tilt_rad; // At year_fraction = 0.25 (Summer Solstice)

        let calculated_latitude_degrees;
        let calculated_year_fraction;

        if tilt_rad.abs() < f32::EPSILON {
            // Special case: Tilt is 0. Declination is always 0.
            // cos(HA) = -tan(latitude) * tan(0) = 0.
            // This implies HA = PI/2, which means day_fraction = 0.5 (12h day/12h night).
            if (day_fraction - 0.5).abs() > f32::EPSILON {
                warn!("TimedSkyCenter on entity {:?} requests a non-12/12 day/night cycle with 0 tilt. This is impossible. Setting 12/12 cycle at equator.", entity);
                 calculated_latitude_degrees = 0.0;
                 calculated_year_fraction = 0.0; // Equinox
            } else {
                 info!("TimedSkyCenter on entity {:?} requests 12/12 day/night with 0 tilt. Setting equator latitude.", entity);
                 calculated_latitude_degrees = 0.0;
                 calculated_year_fraction = 0.0; // Equinox
            }
        } else {
             // General case: Tilt > 0
             let tan_declination = declination_rad.tan();

             // Avoid division by zero if tan_declination is near zero (only happens if tilt is near 0, handled above)
             // or if required_cos_ha is near zero (happens for 12/12 cycle).
             let tan_latitude = if required_cos_ha.abs() < f32::EPSILON {
                 // If cos(HA) is 0, tan(latitude) must be 0 (unless tan(declination) is infinite - poles).
                 // This corresponds to 12/12 day/night, which happens at the equator (lat=0).
                 0.0
             } else if tan_declination.abs() < f32::EPSILON {
                  // This case should be caught by tilt_rad.abs() < f32::EPSILON, but double check
                 warn!("Unexpected near-zero tan_declination encountered for entity {:?}. Defaulting to equator.", entity);
                 0.0
             }
             else {
                 -required_cos_ha / tan_declination
             };

             calculated_latitude_degrees = tan_latitude.atan() * RADIANS_TO_DEGREES;
             calculated_year_fraction = 0.25; // We calculated for Summer Solstice
        }

        info!("Calculated SkyCenter parameters for entity {:?}: Latitude {:.2}°, Year Fraction {:.2}",
              entity, calculated_latitude_degrees, calculated_year_fraction);


        // Create the SkyCenter component
        let sky_center = SkyCenter {
            latitude_degrees: calculated_latitude_degrees,
            planet_tilt_degrees: timed_config.planet_tilt_degrees, // Use the configured tilt
            year_fraction: calculated_year_fraction,
            cycle_duration_secs: total_duration_secs,
            sun: timed_config.sun,
            current_cycle_time: 0.0, // Start cycle at midnight
        };

        if let Some(mut sky_center_target) = sky_center_option {
            sky_center_target.latitude_degrees = calculated_latitude_degrees;
            sky_center_target.year_fraction = calculated_year_fraction;
            sky_center_target.cycle_duration_secs = total_duration_secs;
            sky_center_target.sun = timed_config.sun;
            sky_center_target.planet_tilt_degrees = timed_config.planet_tilt_degrees;
        } else {
            // Replace TimedSkyCenter with SkyCenter and ensure Transform/Visibility are present (handled by SkyCenter's requirements)
            commands.entity(entity)
                .insert(sky_center);
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
    let sin_alt = latitude_rad.sin() * dec_rad.sin() + latitude_rad.cos() * dec_rad.cos() * local_hour_angle_rad.cos();
    let alt_rad = sin_alt.asin(); // Altitude, angle from horizon

    // X (east) component = cos(altitude) * sin(azimuth from North towards East)
    // Z (north) component = cos(altitude) * cos(azimuth from North towards East)
    // We can get these components directly without calculating azimuth explicitly:
    let x_east = dec_rad.cos() * local_hour_angle_rad.sin();
    let z_north = latitude_rad.cos() * dec_rad.sin() - latitude_rad.sin() * dec_rad.cos() * local_hour_angle_rad.cos();

    // Construct the direction vector in the observer's local Bevy frame (X east, Y up, Z north)
    let sun_direction_local = Vec3::new(
        x_east,     // X: East
        sin_alt,    // Y: Up (sin_alt is already calculated)
        z_north,    // Z: North
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
        // Some sky sphere rotation
        let celestial_pole_axis_local = Vec3::new(
            0.0, // Нет компонента в направлении Восток/Запад
            latitude_rad.sin(), // Компонент "вверх" равен sin(широты)
            latitude_rad.cos(), // Компонент "на север" равен cos(широты)
        );
        
        // Вращение небесной сферы
        let rotation_angle_rad = PI - hour_fraction * 2.0 * PI;
        sky_transforms.rotation = Quat::from_axis_angle(celestial_pole_axis_local, rotation_angle_rad);

        let sun_direction_local = calculate_sun_direction(
            hour_fraction,
            latitude_rad,
            tilt_rad,
            year_fraction,
        );

        if let Ok(mut sun_transform) = q_sun.get_mut(sky_center.sun) {
            // The sun's translation in Bevy is interpreted as the vector FROM the origin TOWARDS the light source.
            // The DirectionalLight's direction is -Transform.local_z().
            // So, setting translation to the sun_direction_local and using look_at(ZERO, Y) aligns
            // the light's local -Z axis (its direction) to point from the sun's position (translation)
            // back towards the origin (observer).
            sun_transform.translation = sun_direction_local;
            sun_transform.look_at(Vec3::ZERO, Vec3::Y); // Ensure the light points towards the origin
        }
    }
}


pub fn calculate_timed_sky_center_params(
    planet_tilt_degrees: f32,
    day_duration_secs: f32,
    night_duration_secs: f32,
) -> Option<(f32, f32)> {
    let total_duration_secs = day_duration_secs + night_duration_secs;
    let tilt_rad = planet_tilt_degrees * DEGREES_TO_RADIANS;

    if total_duration_secs <= 0.0 || day_duration_secs < 0.0 || night_duration_secs < 0.0 {
         warn!("Invalid timed durations: day={}s, night={}s. Cannot calculate.", day_duration_secs, night_duration_secs);
         return None;
    }

    if day_duration_secs == 0.0 && night_duration_secs > 0.0 {
        // Perpetual night
        // Requires latitude such that sun never rises (altitude always < 0).
        // At Summer Solstice (dec=tilt), sin(alt) = sin(lat)sin(tilt) + cos(lat)cos(tilt)cos(HA).
        // For perpetual night, min altitude (at noon, HA=0) must be < 0.
        // sin(lat)sin(tilt) + cos(lat)cos(tilt) < 0
        // cos(lat - tilt) < 0
        // This requires lat - tilt > PI/2 (90 degrees) or lat - tilt < -PI/2 (-90 degrees).
        // i.e., lat > tilt + 90 or lat < tilt - 90. Since lat is -90 to 90, this implies lat > 90 or lat < -90.
        // This state is only truly possible at poles if tilt allows sun to circle horizon.
        // For tilt > 0, this means lat must be polewards of 90-tilt.
        // To guarantee no sun at summer solstice (max declination), lat must be > 90 - tilt.
        let min_latitude_for_perpetual_night = 90.0 - planet_tilt_degrees;
        if min_latitude_for_perpetual_night > 90.0 { // Impossible for Earth-like tilts
             warn!("Perpetual night with tilt {} is impossible below poles.", planet_tilt_degrees);
             return None;
        }
         // Choose the northern polewards latitude that ensures perpetual night at summer solstice
         let calculated_latitude_degrees = (90.0 - tilt_rad.abs() * RADIANS_TO_DEGREES).copysign(-tilt_rad.sin()); // Choose the pole that has night

         // A day duration of exactly 0 is ambiguous for year_fraction.
         // Let's return None as this requires special handling (pole setup).
         warn!("Perpetual night requires polar setup. Returning None for general calculation.");
         return None;

    }

     if night_duration_secs == 0.0 && day_duration_secs > 0.0 {
        // Perpetual day
        // Requires latitude such that sun never sets (altitude always > 0).
        // At Summer Solstice (dec=tilt), min altitude (at midnight, HA=PI) must be > 0.
        // sin(lat)sin(tilt) - cos(lat)cos(tilt) > 0
        // -cos(lat + tilt) > 0 => cos(lat + tilt) < 0
        // This requires lat + tilt > PI/2 or lat + tilt < -PI/2.
        // i.e., lat > 90 - tilt or lat < -90 - tilt.
        // Choose the northern polewards latitude that ensures perpetual day at summer solstice
         let min_latitude_for_perpetual_day = 90.0 - planet_tilt_degrees;
         if min_latitude_for_perpetual_day < -90.0 { // Impossible for Earth-like tilts
             warn!("Perpetual day with tilt {} is impossible below poles.", planet_tilt_degrees);
             return None;
         }
         // Choose the northern polewards latitude that ensures perpetual day at summer solstice
         let calculated_latitude_degrees = (90.0 - tilt_rad.abs() * RADIANS_TO_DEGREES).copysign(tilt_rad.sin()); // Choose the pole that has day

         // A night duration of exactly 0 is ambiguous for year_fraction.
         // Let's return None as this requires special handling (pole setup).
         warn!("Perpetual day requires polar setup. Returning None for general calculation.");
         return None;

     }


    let day_fraction = day_duration_secs / total_duration_secs;

    // The magnitude of the Hour Angle (angle from meridian) at sunrise/sunset
    // is PI * (day_fraction).
    // Ref: cos(HA) = -tan(latitude) * tan(declination)
    // We choose Summer Solstice (year_fraction = 0.25) for simplicity,
    // where Declination = Planet Tilt.
    let hour_angle_at_sunset_rad = PI * day_fraction; // Hour angle magnitude from noon to sunset
    let required_cos_ha = hour_angle_at_sunset_rad.cos(); // cos(Hour Angle at sunset/sunrise)
    let declination_rad = tilt_rad; // At year_fraction = 0.25 (Summer Solstice)

    let calculated_latitude_degrees;
    let calculated_year_fraction = 0.25; // We calculate for Summer Solstice

    if tilt_rad.abs() < f32::EPSILON {
        // Special case: Tilt is 0. Declination is always 0.
        // cos(HA) = -tan(latitude) * tan(0) = 0.
        // This implies HA = PI/2, which means day_fraction = 0.5 (12h day/12h night).
        if (day_fraction - 0.5).abs() > f32::EPSILON {
            warn!("Cannot achieve day fraction {} with 0 tilt. Tilt=0 forces 0.5 day fraction.", day_fraction);
             return None; // Impossible
        } else {
             info!("Achieving 12/12 day/night with 0 tilt requires equator latitude.");
             calculated_latitude_degrees = 0.0;
        }
    } else {
         // General case: Tilt > 0
         let tan_declination = declination_rad.tan();

         // cos(HA) = -tan(lat) * tan(dec)
         // tan(lat) = -cos(HA) / tan(dec)
         // This only works if tan(dec) is not zero (tilt not zero) and cos(HA) is not zero (day fraction not 0.5)
         // If cos(HA) is near zero (day fraction near 0.5), tan(lat) is near zero, latitude is near 0.
         // If tan(dec) is near zero (tilt near zero), tan(lat) is very large for non-zero cos(HA), implies latitude near 90/-90.
         // The formula tan(lat) = -cos(HA) / tan(dec) handles these limits via float behavior,
         // but explicit checks are safer for impossible values (e.g. cos(HA) < -tan(dec)).
         // Note: abs(cos(HA)) must be <= abs(tan(dec)) * infinity, which is always true unless tan(dec) is zero.
         // More critically, abs(cos(HA)) must be <= abs(tan(lat) * tan(dec)).
         // abs(tan(lat)) is >= 0. abs(tan(dec)) >= 0.
         // If tan(lat) and tan(dec) have opposite signs, we need cos(HA) > 0 (HA < PI/2 or HA > 3PI/2).
         // If tan(lat) and tan(dec) have same signs, we need cos(HA) < 0 (PI/2 < HA < 3PI/2).
         // This corresponds to whether lat and dec are in same/opposite hemispheres.
         // Our chosen HA is PI * day_fraction, which ranges 0 to PI. cos(HA) ranges 1 to -1.
         // cos(PI * day_fraction) = -tan(lat) * tan(tilt).
         // If day_fraction < 0.5, cos is positive. Requires tan(lat) and tan(tilt) opposite signs (different hemispheres).
         // If day_fraction > 0.5, cos is negative. Requires tan(lat) and tan(tilt) same signs (same hemisphere).
         // This is expected: longer days in hemisphere tilted towards sun.

         let required_tan_latitude = -required_cos_ha / tan_declination;

         // Check if required_tan_latitude is within representable range for atan.
         // It should be if cos(HA) is achievable for *some* latitude (-inf to inf).
         // The only real limitation is |cos(HA)| <= |tan(lat)| * |tan(dec)| for some lat.
         // Since tan(lat) can be any real number, this formula works as long as tan(dec) is not zero.
         calculated_latitude_degrees = required_tan_latitude.atan() * RADIANS_TO_DEGREES;

         // Ensure calculated latitude is within -90 to 90.
         if calculated_latitude_degrees.abs() > 90.0 + f32::EPSILON {
            warn!("Calculation resulted in impossible latitude {:.2}° for tilt {}° and day fraction {:.2}. Returning None.",
                   calculated_latitude_degrees, planet_tilt_degrees, day_fraction);
             return None;
         }
    }

    info!("Calculated parameters: Latitude {:.2}°, Year Fraction {:.2}", calculated_latitude_degrees, calculated_year_fraction);

    Some((calculated_latitude_degrees, calculated_year_fraction))
}