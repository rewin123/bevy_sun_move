use std::f32::consts::PI;

use bevy::{
    core_pipeline::{bloom::Bloom, tonemapping::Tonemapping},
    pbr::{light_consts::lux, Atmosphere, AtmosphereSettings, CascadeShadowConfigBuilder, AmbientLight},
    prelude::*,
    render::camera::Exposure,
};
use bevy_sun_move::*;
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use egui_plot::{Line, Plot, PlotPoints};



fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(SunMovePlugin)
        .add_plugins(EguiPlugin {
            enable_multipass_for_primary_context: false,
        })

        .add_systems(Startup, (setup_camera_fog, setup_terrain_scene))
        // .add_systems(Update, ui_system)
        // .insert_resource(SunMoveConfig {
        //     // London coordinates
        //     latitude_degrees: 51.5,
        //     day_of_year: 172.0,
        //     daynight_duration_secs: 60.0, // smaller for testing
        //     ..Default::default()
        // })
        .run();
}

fn setup_camera_fog(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        // HDR is required for atmospheric scattering to be properly applied to the scene
        Camera {
            hdr: true,
            ..default()
        },
        Transform::from_xyz(-1.2, 0.15, 0.0).looking_at(Vec3::Y * 0.1, Vec3::Y),
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
) {
    // Configure a properly scaled cascade shadow map for this scene (defaults are too large, mesh units are in km)
    let cascade_shadow_config = CascadeShadowConfigBuilder {
        first_cascade_far_bound: 0.3,
        maximum_distance: 3.0,
        ..default()
    }
    .build();

    // Sun
    let id =commands.spawn((
        DirectionalLight {
            shadows_enabled: true,
            illuminance: lux::RAW_SUNLIGHT,
            ..default()
        },
        Transform::default(),
        cascade_shadow_config,
    )).id();

    let sky_center = commands.spawn((
        SkyCenter {
            sun: id,
            latitude_degrees: 90.0, // Approximate latitude of New York
            cycle_duration_secs: 10.0,
            ..default()
        },
        Transform::default(),
    )).id();

    let sphere_mesh = meshes.add(Mesh::from(Sphere { radius: 1.0 }));

    // light probe spheres
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
        Terrain,
        SceneRoot(
            asset_server.load(GltfAssetLabel::Scene(0).from_asset("terrain.glb")),
        ),
        Transform::from_xyz(-1.0, 0.0, -0.5)
            .with_scale(Vec3::splat(0.5))
            .with_rotation(Quat::from_rotation_y(PI / 2.0)),
    ));
}

// fn ui_system(
//     mut contexts: EguiContexts,
//     mut sun_move: ResMut<SunMoveConfig>,
//     sun_info: Res<SunInfo>,
//     ambient_light: Res<AmbientLight>,
// ) {
//     egui::Window::new("Sun Controls & Info").show(contexts.ctx_mut(), |ui| {
//         ui.heading("Sun Parameters");
//         ui.add(egui::Slider::new(&mut sun_move.latitude_degrees, -90.0..=90.0).text("Latitude (°)"));
//         ui.add(egui::Slider::new(&mut sun_move.day_of_year, 1.0..=365.0).text("Day of Year"));
//         ui.add(egui::Slider::new(&mut sun_move.axial_tilt_degrees, 0.0..=90.0).text("Axial Tilt (°)"));
//         ui.add(egui::Slider::new(&mut sun_move.daynight_duration_secs, 1.0..=1200.0).text("Day/Night Duration (s)"));

//         ui.separator();

//         ui.heading("Current Sun Info");
//         ui.label(format!("Azimuth: {:.1}°", sun_info.azimuth_rad * RADIANS_TO_DEGREES));
//         ui.label(format!("Altitude: {:.1}°", sun_info.altitude_rad * RADIANS_TO_DEGREES));
//         ui.label(format!("Intensity Factor: {:.3}", sun_info.intensity_factor));

//         ui.separator();

//         ui.heading("Current Ambient Light Info");
//         let current_ambient_color = ambient_light.color;
//         let current_ambient_brightness = ambient_light.brightness;
//         let color_rgba = current_ambient_color.to_srgba();
//         ui.label(format!(
//             "Ambient Color (RGBA): {:.2}, {:.2}, {:.2}, {:.2}",
//             color_rgba.red, color_rgba.green, color_rgba.blue, color_rgba.alpha
//         ));
//         ui.label(format!("Ambient Brightness: {:.0} Lux", current_ambient_brightness));

//         // Plot Data Calculation
//         let n_points = 100;
//         let latitude_rad = sun_move.latitude_degrees * DEGREES_TO_RADIANS;
//         let axial_tilt_rad = sun_move.axial_tilt_degrees * DEGREES_TO_RADIANS;
//         let day_of_year = sun_move.day_of_year;

//         let altitude_points: PlotPoints = (0..=n_points)
//             .map(|i| {
//                 let time_fraction = i as f32 / n_points as f32;
//                 let (_, altitude_rad, _) = calculate_sun_properties(
//                     time_fraction, latitude_rad, axial_tilt_rad, day_of_year
//                 );
//                 [time_fraction as f64, (altitude_rad * RADIANS_TO_DEGREES) as f64]
//             })
//             .collect();

//         let azimuth_points: PlotPoints = (0..=n_points)
//             .map(|i| {
//                 let time_fraction = i as f32 / n_points as f32;
//                 let (azimuth_rad, _, _) = calculate_sun_properties(
//                     time_fraction, latitude_rad, axial_tilt_rad, day_of_year
//                 );
//                 [time_fraction as f64, (azimuth_rad * RADIANS_TO_DEGREES) as f64]
//             })
//             .collect();

//         let intensity_points: PlotPoints = (0..=n_points)
//             .map(|i| {
//                 let time_fraction = i as f32 / n_points as f32;
//                 let (_, _, intensity_factor) = calculate_sun_properties(
//                     time_fraction, latitude_rad, axial_tilt_rad, day_of_year
//                 );
//                 [time_fraction as f64, intensity_factor as f64]
//             })
//             .collect();

//         let ambient_brightness_points: PlotPoints = (0..=n_points)
//             .map(|i| {
//                 let time_fraction = i as f32 / n_points as f32;
//                 let (_, _, intensity_factor) = calculate_sun_properties(
//                     time_fraction, latitude_rad, axial_tilt_rad, day_of_year
//                 );
//                  let (_, ambient_brightness_factor) = calculate_ambient_light_properties(intensity_factor);
//                 // Use the factor (0..1) for the plot
//                 [time_fraction as f64, ambient_brightness_factor as f64]
//             })
//             .collect();

//         // Ambient Color gradient (approximated for plot background)
//         let ambient_colors: Vec<egui::Color32> = (0..=n_points)
//             .map(|i| {
//                 let time_fraction = i as f32 / n_points as f32;
//                 let (_, _, intensity_factor) = calculate_sun_properties(
//                     time_fraction, latitude_rad, axial_tilt_rad, day_of_year
//                 );
//                 let (color, _) = calculate_ambient_light_properties(intensity_factor);
//                 let srgb = color.to_srgba();
//                 egui::Color32::from_rgba_premultiplied(
//                     (srgb.red * 255.0) as u8,
//                     (srgb.green * 255.0) as u8,
//                     (srgb.blue * 255.0) as u8,
//                     (srgb.alpha * 255.0) as u8,
//                 )
//             })
//             .collect();

//         ui.separator();
//         ui.heading("Sun Trajectory & Intensity Plots (vs Time Fraction)");

//         let altitude_line = Line::new("Altitude (°)", altitude_points);
//         let azimuth_line = Line::new("Azimuth (°)", azimuth_points);
//         let intensity_line = Line::new("Sun Intensity Factor", intensity_points);
//         Plot::new("sun_trajectory_plot")
//             .legend(egui_plot::Legend::default())
//             .view_aspect(2.0)
//             .show(ui, |plot_ui| {
//                 plot_ui.line(altitude_line);
//                 plot_ui.line(azimuth_line);
//             });

//         Plot::new("sun_intensity_plot")
//             .legend(egui_plot::Legend::default())
//             .view_aspect(2.0)
//             .show(ui, |plot_ui| {
//                 plot_ui.line(intensity_line);
//             });


//         ui.separator();
//         ui.heading("Ambient Light Plots (vs Time Fraction)");

//          // Ambient Brightness Plot
//         let ambient_brightness_line = Line::new("Ambient Brightness Factor", ambient_brightness_points);
//         Plot::new("ambient_brightness_plot")
//             .legend(egui_plot::Legend::default())
//             .view_aspect(2.0)
//             .show(ui, |plot_ui| {
//                 plot_ui.line(ambient_brightness_line);
//             });

//         // Ambient Color Gradient Visualization
//         ui.label("Ambient Color Gradient:");
//         let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 20.0), egui::Sense::hover());
//         let painter = ui.painter();
//         let n_colors = ambient_colors.len().max(1);
//         for i in 0..n_colors {
//             let t0 = i as f32 / n_colors as f32;
//             let t1 = (i + 1) as f32 / n_colors as f32;
//             painter.rect_filled(
//                 egui::Rect::from_min_max(
//                     rect.min + egui::vec2(rect.width() * t0, 0.0),
//                     rect.min + egui::vec2(rect.width() * t1, rect.height()),
//                 ),
//                 0.0, // No rounding
//                 ambient_colors[i],
//             );
//         }

//     });
// }