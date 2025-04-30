use std::f32::consts::PI;

use bevy::{
    core_pipeline::{bloom::Bloom, tonemapping::Tonemapping},
    gltf::GltfAssetLabel,
    pbr::{
        Atmosphere, AtmosphereSettings, CascadeShadowConfigBuilder,
        light_consts::lux,
    },
    prelude::*,
    render::{camera::Exposure, mesh::Mesh3d},
    scene::SceneRoot,
};
use bevy_egui::{EguiContexts, EguiPlugin, egui};
use bevy_sun_move::{random_stars::*, *};
use egui_plot::{Line, Plot};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(SunMovePlugin)
        .add_plugins(RandomStarsPlugin)
        .add_plugins(EguiPlugin {
            enable_multipass_for_primary_context: false,
        })
        .add_systems(Startup, (setup_camera_fog, setup_terrain_scene))
        .add_systems(Update, ui_system)
        .run();
}

fn setup_camera_fog(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-1.2, 0.15, 0.0).looking_at(Vec3::Y * 0.1, Vec3::Y),
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

#[derive(Component)]
struct Terrain;

fn setup_terrain_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    let cascade_shadow_config = CascadeShadowConfigBuilder {
        first_cascade_far_bound: 0.3,
        maximum_distance: 3.0,
        ..default()
    }
    .build();

    // Солнце
    let sun_id = commands
        .spawn((
            DirectionalLight {
                shadows_enabled: true,
                illuminance: lux::RAW_SUNLIGHT,
                ..default()
            },
            Transform::default(),
            cascade_shadow_config,
        ))
        .id();

    let sky_config = TimedSkyConfig {
        sun_entity: sun_id,
        planet_tilt_degrees: 23.5, // Earth tilt
        day_duration_secs: 10.0,
        night_duration_secs: 10.0,
        max_sun_height_deg: 45.0, // Usual value for pretty shadow in middle of the day
    };

    commands.spawn((
        sky_config.clone(),
        SkyCenter::from_timed_config(&sky_config).unwrap(),
        Transform::default(),
        Visibility::Visible,
        StarSpawner {
            star_count: 1000,
            spawn_radius: 5000.0,
        },
    ));

    let sphere_mesh = meshes.add(Mesh::from(Sphere { radius: 1.0 }));

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
        SceneRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset("terrain.glb"))),
        Transform::from_xyz(-1.0, 0.0, -0.5)
            .with_scale(Vec3::splat(0.5))
            .with_rotation(Quat::from_rotation_y(PI / 2.0)),
    ));

    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(0.02))),
        MeshMaterial3d(materials.add(Color::srgb(1.0, 0.0, 0.0))),
    ));
}

// --- Система UI ---
fn ui_system(
    mut contexts: EguiContexts,
    mut commands: Commands,
    mut q_sky_entity: Query<(Entity, &mut TimedSkyConfig, Option<&mut SkyCenter>)>,
    q_sun_transform: Query<&Transform, Without<SkyCenter>>,
) {
    let (entity, mut timed_config, mut sky_center_option) = match q_sky_entity.single_mut() {
        Ok(data) => data,
        Err(_) => return,
    };

    egui::Window::new("Sky Cycle Settings").show(contexts.ctx_mut(), |ui| {
        ui.heading("Timed Sky Config");
        ui.label("Configure desired day/night durations and max sun height.");

        ui.add(egui::Slider::new(&mut timed_config.planet_tilt_degrees, 0.0..=90.0).text("Planet Tilt (°)"));
        ui.add(egui::Slider::new(&mut timed_config.day_duration_secs, 0.0..=120.0).text("Desired Day Duration (s)"));
        ui.add(egui::Slider::new(&mut timed_config.night_duration_secs, 0.0..=120.0).text("Desired Night Duration (s)"));
        ui.add(egui::Slider::new(&mut timed_config.max_sun_height_deg, 0.0..=90.0).text("Desired Max Sun Height (°)")); // New slider

        ui.separator();

        // Calculate *potential* resulting parameters based on current TimedSkyConfig values
        let calculation_result = calculate_latitude_yearfraction(
            timed_config.planet_tilt_degrees,
            timed_config.day_duration_secs,
            timed_config.night_duration_secs,
            timed_config.max_sun_height_deg,
        );

        ui.heading("Calculated Parameters");
        if let Some((lat, year, dec)) = calculation_result {
             ui.label(egui::RichText::new(format!("Required Latitude: {:.2}°", lat)).size(18.0));
             ui.label(egui::RichText::new(format!("Resulting Declination: {:.2}°", dec)).size(18.0));
             ui.label(egui::RichText::new(format!("Required Year Fraction: {:.4}", year)).size(18.0));
             ui.label(egui::RichText::new(format!("Total Cycle Duration: {:.2} s", timed_config.day_duration_secs + timed_config.night_duration_secs)).size(18.0));

             if ui.button("Apply Config").clicked() {
                 let total_duration = timed_config.day_duration_secs + timed_config.night_duration_secs;
                 let new_sky_center = SkyCenter {
                     latitude_degrees: lat,
                     planet_tilt_degrees: timed_config.planet_tilt_degrees, // Use configured tilt
                     year_fraction: year,
                     cycle_duration_secs: total_duration,
                     sun: timed_config.sun_entity,
                     current_cycle_time: 0.0, // Reset time to midnight when applying
                 };

                 if let Some(sky_center) = sky_center_option.as_mut() {
                    // Rewrite the existing SkyCenter
                    sky_center.latitude_degrees = lat;
                    sky_center.planet_tilt_degrees = timed_config.planet_tilt_degrees;
                    sky_center.year_fraction = year;
                    sky_center.cycle_duration_secs = total_duration;
                    sky_center.sun = timed_config.sun_entity;
                 } else {
                    commands.entity(entity).insert(new_sky_center);
                 }

                 info!("Applied new SkyCenter settings: Lat {:.2}°, Dec {:.2}°, YF {:.4}, Cycle {:.2}s", lat, dec, year, total_duration);
             }
        } else {
             ui.label(egui::RichText::new("Cannot calculate parameters for this configuration.").color(egui::Color32::RED));
             // Provide hints based on common issues
             let total = timed_config.day_duration_secs + timed_config.night_duration_secs;
             if total <= 0.0 {
                 ui.label(egui::RichText::new("Error: Total duration must be positive.").color(egui::Color32::RED));
             } else if timed_config.max_sun_height_deg < 0.0 || timed_config.max_sun_height_deg > 90.0 {
                  ui.label(egui::RichText::new("Error: Max height must be between 0 and 90 degrees.").color(egui::Color32::RED));
             } else if timed_config.planet_tilt_degrees.abs() < f32::EPSILON && (timed_config.day_duration_secs / total - 0.5).abs() > f32::EPSILON {
                  ui.label(egui::RichText::new("Error: With 0 tilt, day/night must be equal duration (12/12).").color(egui::Color32::RED));
             } else if timed_config.day_duration_secs < f32::EPSILON && timed_config.max_sun_height_deg > f32::EPSILON {
                  ui.label(egui::RichText::new("Error: Perpetual night is requested, but max height > 0 is impossible.").color(egui::Color32::RED));
             } else if timed_config.night_duration_secs < f32::EPSILON && timed_config.max_sun_height_deg < f32::EPSILON {
                 ui.label(egui::RichText::new("Error: Perpetual day is requested, but max height <= 0 is impossible (unless 12/12 max height 0).").color(egui::Color32::RED));
             }
              else {
                  ui.label(egui::RichText::new("Impossible combination of day/night ratio and max height for the given tilt.").color(egui::Color32::RED));
                  ui.label(egui::RichText::new("Try adjusting tilt, durations, or max height.").color(egui::Color32::RED));
              }
        }

        ui.separator();

        // --- Display information from the active SkyCenter (if present) ---
        ui.heading("Current Active Sky Center Info");
        if let Some(mut sky_center) = sky_center_option { // Need mut to allow slider changes

            ui.label(format!("Actual Latitude: {:.2}°", sky_center.latitude_degrees));
            ui.label(format!("Actual Planet Tilt: {:.2}°", sky_center.planet_tilt_degrees));
            ui.label(format!("Actual Year Fraction: {:.4}", sky_center.year_fraction));
             let actual_dec_deg = sky_center.planet_tilt_degrees * (sky_center.year_fraction * 2.0 * PI).sin() * RADIANS_TO_DEGREES;
             ui.label(format!("Actual Declination: {:.2}°", actual_dec_deg));
            ui.add(egui::Slider::new(&mut sky_center.cycle_duration_secs, 0.0..=120.0).text("Actual Cycle Duration (s)")); // Allow changing actual duration

            // Pause/Play option
            let is_paused = sky_center.cycle_duration_secs <= f32::EPSILON;
            let pause_text = if is_paused { "Play" } else { "Pause" };
            if ui.button(pause_text).clicked() {
                 if is_paused {
                      // Restore a default value or previous value? Let's use 30s if it was 0
                      if sky_center.cycle_duration_secs <= f32::EPSILON {
                         sky_center.cycle_duration_secs = 30.0;
                      }
                       // Ensure current_cycle_time is a valid fraction if it was a fraction
                      sky_center.current_cycle_time = sky_center.current_cycle_time.fract().max(0.0);
                       if sky_center.cycle_duration_secs > f32::EPSILON { // Normalize if duration is now positive
                          sky_center.current_cycle_time *= sky_center.cycle_duration_secs;
                       }
                 } else {
                     // Pause: Store the current state as a fraction [0, 1)
                      if sky_center.cycle_duration_secs > f32::EPSILON {
                        sky_center.current_cycle_time /= sky_center.cycle_duration_secs;
                      } // else it might already be a fraction if it was paused before
                      sky_center.cycle_duration_secs = 0.0; // Pause
                 }
            }

            // Time slider for active cycle
            let hour_fraction = if sky_center.cycle_duration_secs > f32::EPSILON {
                 sky_center.current_cycle_time / sky_center.cycle_duration_secs
            } else {
                 sky_center.current_cycle_time.clamp(0.0, 1.0) // Treat as fraction 0-1 when paused
            };

            if sky_center.cycle_duration_secs > f32::EPSILON {
                let cycle_duration = sky_center.cycle_duration_secs;
                 ui.add(egui::Slider::new(&mut sky_center.current_cycle_time, 0.0..=cycle_duration).text("Current Cycle Time (s)"))
            } else {
                // Slide using hour representation when paused (current_cycle_time stores the fraction)
                 let mut display_fraction = hour_fraction;
                 let response = ui.add(egui::Slider::new(&mut display_fraction, 0.0..=1.0).text("Current Cycle Fraction (0-1)"));
                 if response.changed() {
                     sky_center.current_cycle_time = display_fraction.clamp(0.0, 1.0);
                 }
                 response
            };

            // Show time of day in 24hr format
            if !is_paused {
                 ui.label(format!("Time of Day: {:02.0}:{:02.0} ({:.2} hours)",
                    (hour_fraction * 24.0) as u32,
                    ((hour_fraction * 24.0).fract() * 60.0) as u32,
                    hour_fraction * 24.0
                 ));
            } else {
                 ui.label(format!("Time of Day: {:.2} hours (Paused)", hour_fraction * 24.0));
            }


            // Get current sun info from its transform
            ui.separator();
            ui.heading("Current Sun Info");
            let sun_transform_actual = q_sun_transform.get(sky_center.sun).ok();


            if let Some(sun_transform) = sun_transform_actual { 
                 let current_sun_direction = sun_transform.translation.normalize(); 

                 let elevation_rad = current_sun_direction.y.asin(); // Y is Up
                 let elevation_degrees = elevation_rad * RADIANS_TO_DEGREES;
                 ui.label(format!("Sun Elevation: {:.1}°", elevation_degrees));

                 // X is East, Z is North. Azimuth from North towards East.
                 let horizontal_direction = Vec2::new(current_sun_direction.x, current_sun_direction.z);
                 let heading_rad = horizontal_direction.x.atan2(horizontal_direction.y); // atan2(East, North)
                 let mut heading_degrees = heading_rad * RADIANS_TO_DEGREES;
                 if heading_degrees < 0.0 { heading_degrees += 360.0; } // Normalize 0-360
                  ui.label(format!("Sun Heading (from North): {:.1}°", heading_degrees));


                  ui.separator();
                  ui.heading("Sun Trajectory Plot (Active Settings)");

                  let n_points = 100;
                  let latitude_rad = sky_center.latitude_degrees * DEGREES_TO_RADIANS;
                  let axial_tilt_rad = sky_center.planet_tilt_degrees * DEGREES_TO_RADIANS;
                  let year_fraction = sky_center.year_fraction; // Use actual year fraction

                  let mut sun_elevation_points: Vec<[f64; 2]> = Vec::new();
                  let mut sun_heading_points: Vec<[f64; 2]> = Vec::new();

                  for i in 0..=n_points {
                      let hour_fraction_plot = i as f32 / n_points as f32;
                      let sun_direction = calculate_sun_direction(
                          hour_fraction_plot,
                          latitude_rad,
                          axial_tilt_rad,
                          year_fraction,
                      );

                      let elevation_rad = sun_direction.y.asin();
                      let elevation_degrees = elevation_rad * RADIANS_TO_DEGREES;
                      sun_elevation_points.push([hour_fraction_plot as f64, elevation_degrees as f64]);

                      let horizontal_direction_plot = Vec2::new(sun_direction.x, sun_direction.z);
                      let heading_rad = horizontal_direction_plot.x.atan2(horizontal_direction_plot.y);
                       let mut heading_degrees = heading_rad * RADIANS_TO_DEGREES;
                       if heading_degrees < 0.0 { heading_degrees += 360.0; }
                      sun_heading_points.push([hour_fraction_plot as f64, heading_degrees as f64]);
                  }

                  let sun_elevation_line = Line::new("Elevation (°)", sun_elevation_points);
                  let sun_heading_line = Line::new("Heading (°)", sun_heading_points);

                  Plot::new("sun_trajectory_plot")
                      .legend(egui_plot::Legend::default())
                      .view_aspect(2.0)
                      .set_margin_fraction(egui::vec2(0.1, 0.1))
                      .x_axis_label("Day Fraction (0=Mid, 0.5=Noon, 1=Mid)")
                      .y_axis_label("Angle (°)")
                      .show(ui, |plot_ui| {
                          plot_ui.line(sun_elevation_line);
                          plot_ui.line(sun_heading_line);
                      });

            } else {
                ui.label("Sun entity transform not found.");
            } 

        } else { 
            ui.label("SkyCenter component not active yet. Apply config first.");
        } 

    });
}
