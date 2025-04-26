use bevy::{
    math::Affine3A, pbr::{light_consts::lux, AmbientLight, DirectionalLight}, prelude::*
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





pub fn get_sphere_local_coords(
    latitude_rad: f32,
    longitude_rad: f32,
) -> (Vec3, Vec3, Vec3) {
    let y = latitude_rad.sin();
    let xz = latitude_rad.cos();

    let x = xz * longitude_rad.sin();
    let z = xz * longitude_rad.cos();

    let up = Vec3::new(x, y, z).normalize();
    let east = Vec3::new(longitude_rad.cos(), 0.0, -longitude_rad.sin()).normalize();
    let north = up.cross(east);

    info!("{} {} {}", east, up, north);

    (east, up, north)
}

pub fn get_sphere_quat(
    latitude_rad: f32,
    longitude_rad: f32,
) -> Quat {
    return Quat::from_rotation_y(longitude_rad) * Quat::from_rotation_x(-latitude_rad + PI / 2.0) * Quat::from_rotation_y(PI / 2.0);
}

pub fn get_planet_tilt_quat(
    tilt_rad: f32,
    year_fraction: f32,
) -> Quat {
    return Quat::from_rotation_y(year_fraction * 2.0 * PI) * Quat::from_rotation_x(tilt_rad);
}


#[derive(Component, Debug, Clone)]
#[require(Transform)]
pub struct SkyCenter {
    pub latitude_degrees: f32,
    pub planet_tilt_degrees: f32,

    
    pub year_fraction: f32,

    pub cycle_duration_secs: f32,

    pub sun: Entity,
}

impl Default for SkyCenter {
    fn default() -> Self {
        Self {
            latitude_degrees: 0.0,
            planet_tilt_degrees: 23.5,
            year_fraction: 0.0,
            cycle_duration_secs: 600.0,
            sun: Entity::PLACEHOLDER,
        }
    }
}



fn update_sky_center(
    mut q_sky_center: Query<(&mut SkyCenter, &mut Transform)>,
    mut q_sun: Query<(&mut Transform), Without<SkyCenter>>,
    time: Res<Time>,
) {
    for (mut sky_center, mut transform) in q_sky_center.iter_mut() {

        let hour_fraction = time.elapsed_secs() / sky_center.cycle_duration_secs;

        let planet_quat = get_planet_tilt_quat(sky_center.planet_tilt_degrees * DEGREES_TO_RADIANS, sky_center.year_fraction);
        let sphere_quat = get_sphere_quat(sky_center.latitude_degrees * DEGREES_TO_RADIANS, hour_fraction * 2.0 * PI);
        let world_quat = planet_quat * sphere_quat;

        info!("{}", world_quat);

        if let Ok(mut sun) = q_sun.get_mut(sky_center.sun) {
            sun.translation = world_quat.inverse() * Vec3::X;
            sun.look_at(Vec3::ZERO, Vec3::Y);
        }
    }
}