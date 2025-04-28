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
    }
}


#[derive(Component, Debug, Clone)]
#[require(Transform)]
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