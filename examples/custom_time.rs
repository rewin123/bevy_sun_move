use bevy::{
    camera::Exposure,
    core_pipeline::tonemapping::Tonemapping,
    light::light_consts::lux,
    pbr::{Atmosphere, AtmosphereSettings},
    post_process::bloom::Bloom,
    prelude::*, render::view::Hdr,
};
use bevy_egui::*;
use bevy_sun_move::{random_stars::*, *};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(TypedSunMovePlugin::<Time<CustomTime>>::default())
        .add_plugins(RandomStarsPlugin)
        .add_plugins(EguiPlugin::default())
        .insert_resource(Time::<CustomTime>::default())
        .add_systems(Startup, (setup_camera_fog, setup_terrain_scene))
        .add_systems(Update, update_custom_time)
        .add_systems(EguiPrimaryContextPass, ui_custom_time)
        .run();
}

pub struct CustomTime {
    pub relative_speed: f32,
    last_set_speed: f32, // Speed (and direction) to use when unpausing or changing mode.
}

impl Default for CustomTime {
    fn default() -> Self {
        Self {
            relative_speed: 1.0, // Start playing forward
            last_set_speed: 1.0, // Default play speed is 1.0 forward
        }
    }
}

fn update_custom_time(mut custom_time: ResMut<Time<CustomTime>>, time: Res<Time>) {
    let delta = time.delta().mul_f32(custom_time.context().relative_speed);
    custom_time.advance_by(delta);
}

fn ui_custom_time(
    mut commands: Commands,
    mut custom_time: ResMut<Time<CustomTime>>,
    time: Res<Time>,
    mut egui_context: EguiContexts,
) -> Result {
    const MIN_PLAY_SPEED: f32 = 0.125;
    const MAX_PLAY_SPEED: f32 = 16.0;

    egui::Window::new("Custom Time").show(egui_context.ctx_mut()?, |ui| {
        ui.label(format!(
            "Custom time: {:.2} seconds",
            custom_time.elapsed_secs()
        ));
        ui.label(format!("Time: {:.2} seconds", time.elapsed_secs()));
        ui.label(format!(
            "Relative speed: {:.3}",
            custom_time.context().relative_speed
        ));
        ui.label(format!(
            "Last set speed: {:.3}",
            custom_time.context().last_set_speed
        ));

        if ui.button("Reset").clicked() {
            commands.insert_resource(Time::<CustomTime>::default());
        }

        ui.horizontal(|ui| {
            let ctx = custom_time.context_mut();

            // Slower button
            if ui.button("⏪ Slower").clicked() {
                let mut speed = ctx.relative_speed;
                if speed > MIN_PLAY_SPEED {
                    speed /= 2.0;
                    ctx.relative_speed = speed.max(MIN_PLAY_SPEED);
                    ctx.last_set_speed = ctx.relative_speed;
                } else if speed < -MIN_PLAY_SPEED {
                    // Rewinding
                    speed /= 2.0; // e.g., -4.0 -> -2.0
                    ctx.relative_speed = speed.min(-MIN_PLAY_SPEED); // e.g. min(-0.125)
                    ctx.last_set_speed = ctx.relative_speed;
                }
                // If speed is 0, or between -MIN_PLAY_SPEED and MIN_PLAY_SPEED, Slower does nothing to relative_speed.
            }

            // Play/Pause toggle
            let current_relative_speed = ctx.relative_speed;
            if current_relative_speed == 0.0 {
                if ui.button("▶ Play").clicked() {
                    ctx.relative_speed = ctx.last_set_speed;
                    // If last_set_speed was 0, ensure it starts at a minimum speed
                    if ctx.relative_speed.abs() < MIN_PLAY_SPEED {
                        ctx.relative_speed = if ctx.last_set_speed >= 0.0 {
                            MIN_PLAY_SPEED
                        } else {
                            -MIN_PLAY_SPEED
                        };
                        ctx.last_set_speed = ctx.relative_speed; // also update last_set_speed to this new minimum
                    }
                }
            } else {
                if ui.button("⏸ Pause").clicked() {
                    // Store the current speed (which might have been changed by Slower/Faster) before pausing
                    ctx.last_set_speed = current_relative_speed;
                    ctx.relative_speed = 0.0;
                }
            }

            // Faster button
            if ui.button("⏩ Faster").clicked() {
                let mut speed = ctx.relative_speed;

                if speed == 0.0 {
                    // Paused
                    // Resume with last_set_speed, or MIN_PLAY_SPEED if last_set_speed is too small/zero
                    speed = ctx.last_set_speed;
                    if speed.abs() < MIN_PLAY_SPEED {
                        speed = if speed >= 0.0 {
                            MIN_PLAY_SPEED
                        } else {
                            -MIN_PLAY_SPEED
                        };
                    }
                } else if speed > 0.0 {
                    // Playing forward
                    if speed < MAX_PLAY_SPEED {
                        speed *= 2.0;
                        speed = speed.min(MAX_PLAY_SPEED);
                    }
                } else {
                    // Rewinding (speed < 0.0)
                    if speed > -MAX_PLAY_SPEED {
                        speed *= 2.0; // Makes it more negative
                        speed = speed.max(-MAX_PLAY_SPEED);
                    }
                }
                ctx.relative_speed = speed;
                if ctx.relative_speed.abs() > f32::EPSILON {
                    ctx.last_set_speed = ctx.relative_speed;
                }
            }
        });
    });

    Ok(())
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

fn setup_camera_fog(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-1.2, 0.15, 0.0).looking_at(Vec3::Y * 0.1, Vec3::Y),
        // HDR is required for atmospheric scattering to be properly applied to the scene
        Hdr,
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
