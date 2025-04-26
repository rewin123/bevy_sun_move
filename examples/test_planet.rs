use std::f32::consts::PI;

use bevy::prelude::*;
use bevy_sun_move::*;


fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, point)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 2.0, 2.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(1.0))),
        MeshMaterial3d(materials.add(Color::WHITE)),
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));

    commands.spawn((
        DirectionalLight::default(),
        Transform::from_xyz(1.0, 1.0, -1.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}



fn point(
    time: Res<Time>,
    mut gizmos: Gizmos
) {

    let latitude = 65.0;
    let longitude = time.elapsed_secs() * 100.0;
    let tilt = 23.5 * PI / 180.0;
    // let longitude = 0.0;

    info!("{} N {} E", latitude, longitude);

    let quat = get_sphere_quat(latitude / 180.0 * PI, longitude / 180.0 * PI);
    let tilt_quat = get_planet_tilt_quat(tilt, time.elapsed_secs() / 10.0);
    let quat = tilt_quat * quat;

    // Draw local axes
    let point = quat * Vec3::Y;
    gizmos.line(point, point + quat * Vec3::X, Color::srgb(1.0, 0.0, 0.0));
    gizmos.line(point, point + quat * Vec3::Y, Color::srgb(0.0, 1.0, 0.0));
    gizmos.line(point, point + quat * Vec3::Z, Color::srgb(0.0, 0.0, 1.0));


    // Earth rotation axis
    gizmos.line(Vec3::ZERO, 2.0 * (tilt_quat * Vec3::Y), Color::srgb(1.0, 1.0, 1.0));

}