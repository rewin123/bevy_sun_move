use bevy::{
    core_pipeline::{bloom::Bloom, tonemapping::Tonemapping},
    pbr::{
        Atmosphere, AtmosphereSettings,
        light_consts::lux,
    },
    prelude::*,
    render::{camera::Exposure, mesh::Mesh3d},
};
use bevy_sun_move::{random_stars::*, *};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(SunMovePlugin)
        .add_plugins(RandomStarsPlugin)
        .add_systems(Startup, (setup_camera_fog, setup_terrain_scene))
        .run();
}

fn setup_camera_fog(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-1.2, 0.15, 0.0).looking_at(Vec3::Y * 0.1, Vec3::Y),
        // HDR is required for atmospheric scattering to be properly applied to the scene
        Camera {
            hdr: true,
            ..default()
        },
        Atmosphere::EARTH,
        AtmosphereSettings {
            aerial_view_lut_max_distance: 3.2e5,
            scene_units_to_m: 1e+4,
            ..Default::default()
        },
        Exposure::SUNLIGHT,
        Tonemapping::AcesFitted,
        Bloom::NATURAL,
    ));
}


// Spawn scene similar to the bevy github example
fn setup_terrain_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {

    // Sun
    let sun_id = commands
        .spawn((
            DirectionalLight {
                shadows_enabled: true,
                illuminance: lux::RAW_SUNLIGHT, // Full sunlight illuminance
                ..default()
            },
            Transform::default(),
        ))
        .id();

    let timed_sky_config = TimedSkyConfig {
        sun_entity: sun_id,
        day_duration_secs: 10.0,
        night_duration_secs: 5.0,
        max_sun_height_deg: 45.0,
        ..default()
    };

    // -- Create the SkyCenter entity
    commands.spawn((
        SkyCenter::from_timed_config(&timed_sky_config).unwrap(),
        Visibility::Visible,
        StarSpawner {
            star_count: 1000,
            spawn_radius: 5000.0,
        },
    ));

    let sphere_mesh = meshes.add(Mesh::from(Sphere { radius: 1.0 }));

    // light probe spheres (using Mesh3dBundle for convenience)
    commands.spawn((
        Mesh3d(sphere_mesh.clone()),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::WHITE,
            metallic: 1.0,
            perceptual_roughness: 0.0,
            ..default()
        })),
        Transform::from_xyz(-0.3, 0.1, -0.1).with_scale(Vec3::splat(0.05)),
    ));

    commands.spawn((
        Mesh3d(sphere_mesh.clone()),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::WHITE,
            metallic: 0.0,
            perceptual_roughness: 1.0,
            ..default()
        })),
        Transform::from_xyz(-0.3, 0.1, 0.1).with_scale(Vec3::splat(0.05)),
    ));

    commands.spawn((
        Mesh3d(meshes.add(Plane3d::new(Vec3::Y, Vec2::new(1000.0, 1000.0)))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::WHITE,
            cull_mode: None,
            ..default()
        })),
        Transform::default(),
    ));
}
