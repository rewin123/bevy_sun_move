use std::f32::consts::PI;

use bevy::{
    core_pipeline::{bloom::Bloom, tonemapping::Tonemapping},
    pbr::{light_consts::lux, Atmosphere, AtmosphereSettings, CascadeShadowConfigBuilder, AmbientLight},
    prelude::*,
    render::camera::Exposure,
     render::mesh::{Mesh3d}, // Added missing imports
    scene::SceneRoot, // Added missing imports
    gltf::GltfAssetLabel, // Added missing imports
};
use bevy_sun_move::*; // Your library
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use egui_plot::{Line, Plot, PlotPoints, AxisHints}; // Added AxisHints


fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(SunMovePlugin) // Your plugin
        .add_plugins(EguiPlugin {
            enable_multipass_for_primary_context: false
        })
        .add_systems(Startup, (setup_camera_fog, setup_terrain_scene))
        // .add_systems(Update, (ui_system, update_ambient_light)) // Add ui system and a system to update ambient light
        .add_systems(Update, ui_system)
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
        // This is the component that enables atmospheric scattering for a camera
        Atmosphere::EARTH,
        // The scene is in units of 10km, so we need to scale up the
        // aerial view lut distance and set the scene scale accordingly.
        // Most usages of this feature will not need to adjust this.
        AtmosphereSettings {
            aerial_view_lut_max_distance: 3.2e5,
            scene_units_to_m: 1e+4,
            ..Default::default()
        },
        // The directional light illuminance  used in this scene
        // (the one recommended for use with this feature) is
        // quite bright, so raising the exposure compensation helps
        // bring the scene to a nicer brightness range.
        Exposure::SUNLIGHT,
        // Tonemapper chosen just because it looked good with the scene, any
        // tonemapper would be fine :)
        Tonemapping::AcesFitted,
        // Bloom gives the sun a much more natural look.
        Bloom::NATURAL,
    ));
}

#[derive(Component)]
struct Terrain;


// Spawn same scene as in the bevy github example
fn setup_terrain_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    mut ambient_light: ResMut<AmbientLight>, // Get ambient light resource
) {
    // Configure a properly scaled cascade shadow map for this scene (defaults are too large, mesh units are in km)
    let cascade_shadow_config = CascadeShadowConfigBuilder {
        first_cascade_far_bound: 0.3,
        maximum_distance: 3.0,
        ..default()
    }
    .build();

    // Sun
    let sun_id = commands.spawn((
        DirectionalLight {
            shadows_enabled: true,
            illuminance: lux::RAW_SUNLIGHT, // Full sunlight illuminance
            ..default()
        },
        // Start position doesn't matter as update_sky_center will set it
        Transform::default(),
        cascade_shadow_config,
    )).id();

    // Create the SkyCenter entity
    commands.spawn((
        SkyCenter {
            sun: sun_id,
            latitude_degrees: 51.5, // Approximate latitude of London
            planet_tilt_degrees: 23.5, // Earth's axial tilt
            year_fraction: 0.0, // Vernal Equinox
            cycle_duration_secs: 30.0, // A 30-second day
            current_cycle_time: 0.0, // Start at midnight
            // ambient_light: ambient_light_entity_id, // If you make ambient light a component
            ..default()
        },
        // SkyCenter doesn't need a visible transform, it just holds parameters
        // Transform::default(), // Removed Transform
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


    // Terrain (using SceneBundle for convenience)
    commands.spawn((
        SceneRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset("terrain.glb"))),
        Transform::from_xyz(-1.0, 0.0, -0.5)
            .with_scale(Vec3::splat(0.5))
            .with_rotation(Quat::from_rotation_y(PI / 2.0)),
    ));

    // Add an origin marker sphere
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(0.02))),
        MeshMaterial3d(materials.add(Color::srgb(1.0, 0.0, 0.0))),
    ));
}


// // System to update ambient light based on sun altitude
// fn update_ambient_light(
//     mut ambient_light: ResMut<AmbientLight>,
//     q_sky_center: Query<&SkyCenter>,
//     q_sun_transform: Query<&Transform, Without<SkyCenter>>,
// ) {
//     if let Ok(sky_center) = q_sky_center.single() {
//         if let Ok(sun_transform) = q_sun_transform.get(sky_center.sun) {
//             // Sun direction vector points from origin (observer) towards sun
//             let sun_dir = sun_transform.translation.normalize();

//             // Altitude is the angle above the horizon (Y component in our local frame)
//             let altitude_rad = sun_dir.y.asin();
//             let altitude_degrees = altitude_rad * RADIANS_TO_DEGREES;

//             // Adjust ambient light based on altitude
//             // - Below horizon (< 0 deg): minimal ambient
//             // - Near horizon (0-10 deg): transition to daylight ambient
//             // - Above horizon (> 10 deg): full daylight ambient

//             // Simple lerp or stepped adjustment
//             let day_brightness = 0.5; // Adjust based on desired look
//             let night_brightness = 0.05; // Adjust based on desired look

//             let transition_start_alt = 0.0; // degrees
//             let transition_end_alt = 10.0; // degrees

//             let brightness = if altitude_degrees < transition_start_alt {
//                 night_brightness
//             } else if altitude_degrees > transition_end_alt {
//                 day_brightness
//             } else {
//                 // Linear interpolation within transition band
//                 let factor = (altitude_degrees - transition_start_alt) / (transition_end_alt - transition_start_alt);
//                 night_brightness.lerp(day_brightness, factor)
//             };

//              ambient_light.brightness = brightness;

//              // Optional: adjust ambient color slightly (e.g., warmer near horizon)
//              // ambient_light.color = Color::WHITE; // Keep white for simplicity or change based on altitude/time of day
//         }
//     }
// }


fn ui_system(
    mut contexts: EguiContexts,
    mut q_sky_center: Query<&mut SkyCenter>,
    q_transform: Query<&Transform>,
) {
    // Use get_single_mut() which handles the case where the query is empty or has multiple results
    let mut sky_center = match q_sky_center.get_single_mut() {
        Ok(sc) => sc,
        Err(_) => return, // Exit if SkyCenter entity is not found or is not unique
    };

    egui::Window::new("Sun Controls & Info").show(contexts.ctx_mut(), |ui| {
        ui.heading("Sun Parameters");
        ui.add(egui::Slider::new(&mut sky_center.latitude_degrees, -90.0..=90.0).text("Latitude (°)"));
        ui.add(egui::Slider::new(&mut sky_center.planet_tilt_degrees, 0.0..=90.0).text("Planet Tilt (°)")); // Tilt usually 0-90
        ui.add(egui::Slider::new(&mut sky_center.year_fraction, 0.0..=1.0).text("Year Fraction (0=VE, 0.25=SS, 0.5=AE, 0.75=WS)"));
        ui.add(egui::Slider::new(&mut sky_center.cycle_duration_secs, 1.0..=120.0).text("Day/Night Duration (s)")); // Shorter max duration for faster cycles

        // Option to pause/play time
        let is_paused = sky_center.cycle_duration_secs == 0.0;
        if ui.button(if is_paused { "Play" } else { "Pause" }).clicked() {
            if is_paused {
                 // Restore a default duration if paused
                 sky_center.cycle_duration_secs = 30.0;
                 // Ensure current_cycle_time is within bounds after unpausing
                 sky_center.current_cycle_time %= sky_center.cycle_duration_secs.max(1.0); // Prevent division by zero
            } else {
                 // Store current duration before pausing
                 // (Optional, could just set to 0.0)
                 sky_center.cycle_duration_secs = 0.0; // Pause by setting duration to 0
            }
        }

         if sky_center.cycle_duration_secs > 0.0 { // Only show time slider if not paused
             let mut current_cycle_time = sky_center.current_cycle_time;
             if ui.add(egui::Slider::new(&mut current_cycle_time, 0.0..=sky_center.cycle_duration_secs).text("Current Cycle Time (s)")).changed() {
                 sky_center.current_cycle_time = current_cycle_time;
             }
         }


        ui.separator();

        // Get current sun info from its transform
        let sun_transform = q_transform.get(sky_center.sun).ok(); // Use ok() to handle potential errors

        ui.heading("Current Sun Info");
        if let Some(sun_transform) = sun_transform {
            let current_sun_position = sun_transform.translation.normalize(); // Normalize for direction vector

            // Calculate Elevation (Altitude)
            let elevation_rad = current_sun_position.y.asin(); // Y is up in Bevy local frame
            let elevation_degrees = elevation_rad * RADIANS_TO_DEGREES;
             ui.label(format!("Sun Elevation: {:.1}°", elevation_degrees));


            // Calculate Heading (Azimuth from North towards East)
            // Bevy's X is East, Z is North in our calculation frame
            let heading_rad = current_sun_position.x.atan2(current_sun_position.z);
            let mut heading_degrees = heading_rad * RADIANS_TO_DEGREES;
            // Normalize heading to 0-360 degrees if preferred, or keep -180 to 180
            if heading_degrees < 0.0 {
                heading_degrees += 360.0;
            }
            ui.label(format!("Sun Heading (from North): {:.1}°", heading_degrees));

             let hour_fraction = sky_center.current_cycle_time / sky_center.cycle_duration_secs.max(1.0); // Use max(1.0) to avoid division by zero if paused
             let hour_of_day = hour_fraction * 24.0;
             ui.label(format!("Time of Day: {:.2} hours", hour_of_day));


        } else {
             ui.label("Sun entity not found or query error.");
        }


        ui.separator();

        // Plot Data Calculation
        let n_points = 100;
        let latitude_rad = sky_center.latitude_degrees * DEGREES_TO_RADIANS;
        let axial_tilt_rad = sky_center.planet_tilt_degrees * DEGREES_TO_RADIANS;
        let year_fraction = sky_center.year_fraction;

        let mut sun_elevation_points: Vec<[f64; 2]> = Vec::new();
        let mut sun_heading_points: Vec<[f64; 2]> = Vec::new();

        for i in 0..=n_points {
            let hour_fraction = i as f32 / n_points as f32;
            let sun_direction = calculate_sun_direction(
                hour_fraction,
                latitude_rad,
                axial_tilt_rad,
                year_fraction,
            );

            // Elevation (Altitude) for plot
            let elevation_rad = sun_direction.y.asin();
            let elevation_degrees = elevation_rad * RADIANS_TO_DEGREES;
            sun_elevation_points.push([hour_fraction as f64, elevation_degrees as f64]);

            // Heading (Azimuth from North towards East) for plot
            let heading_rad = sun_direction.x.atan2(sun_direction.z);
            let mut heading_degrees = heading_rad * RADIANS_TO_DEGREES;
            // Normalize heading for plot continuity if needed (-180 to 180 is fine for egui_plot default)
            sun_heading_points.push([hour_fraction as f64, heading_degrees as f64]);
        }

        ui.separator();
        ui.heading("Sun Trajectory (vs Day Fraction)");

        let sun_elevation_line = Line::new("Elevation (°)", sun_elevation_points);
        let sun_heading_line = Line::new("Heading (°)", sun_heading_points);

        Plot::new("sun_trajectory_plot")
            .legend(egui_plot::Legend::default())
            .view_aspect(2.0)
            .set_margin_fraction(egui::vec2(0.1, 0.1)) // Add some margin
            .x_axis_label("Day Fraction (0=Mid, 0.5=Noon, 1=Mid)") // Label X axis
            .y_axis_label("Angle (°)") // Label Y axis
            .show(ui, |plot_ui| {
                plot_ui.line(sun_elevation_line);
                plot_ui.line(sun_heading_line);
            });
    });
}