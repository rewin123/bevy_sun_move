use std::f32::consts::PI;

use bevy::{
    core_pipeline::{auto_exposure::AutoExposure, bloom::Bloom, tonemapping::Tonemapping}, gltf::GltfAssetLabel, pbr::{light_consts::lux, AmbientLight, Atmosphere, AtmosphereSettings, CascadeShadowConfigBuilder, NotShadowCaster}, prelude::*, render::{camera::Exposure, mesh::Mesh3d, render_resource::Face}, scene::SceneRoot // Added missing imports
};
use bevy_sun_move::{random_stars::*, *}; // Your library
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use egui_plot::{Line, Plot, PlotPoints, AxisHints}; // Added AxisHints


fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(SunMovePlugin) // Your plugin
        .add_plugins(RandomStarsPlugin)
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
            year_fraction: 0.0, 
            cycle_duration_secs: 30.0, // A 30-second day
            current_cycle_time: 0.0, // Start at midnight
            ..default()
        },
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