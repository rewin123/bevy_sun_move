// examples/timed_location.rs

use std::f32::consts::PI;

use bevy::{
    core_pipeline::{auto_exposure::AutoExposure, bloom::Bloom, tonemapping::Tonemapping}, gltf::GltfAssetLabel, pbr::{light_consts::lux, AmbientLight, Atmosphere, AtmosphereSettings, CascadeShadowConfigBuilder, NotShadowCaster}, prelude::*, render::{camera::Exposure, mesh::Mesh3d, render_resource::Face}, scene::SceneRoot
};
// Импортируем все необходимое из вашей библиотеки, включая вспомогательные функции и компоненты
use bevy_sun_move::{
    calculate_sun_direction, calculate_timed_sky_center_params, random_stars::*, SkyCenter, SunMovePlugin, TimedSkyCenter, DEGREES_TO_RADIANS, RADIANS_TO_DEGREES
};
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use egui_plot::{Line, Plot, PlotPoints};


fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(SunMovePlugin) // Ваш плагин (теперь без автоматической настройки по времени)
        .add_plugins(RandomStarsPlugin)
        .add_plugins(EguiPlugin {
            enable_multipass_for_primary_context: false
        })
        .add_systems(Startup, (setup_camera_fog, setup_terrain_scene))
        .add_systems(Update, ui_system) // Система UI будет управлять настройками по времени и обновлять SkyCenter
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


// Спавним сцену, аналогичную примеру из github bevy
  fn setup_terrain_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    // Настраиваем каскадную карту теней, масштабированную для этой сцены (единицы меша в км)
    let cascade_shadow_config = CascadeShadowConfigBuilder {
        first_cascade_far_bound: 0.3,
        maximum_distance: 3.0,
        ..default()
    }
    .build();

    // Солнце
    let sun_id = commands.spawn((
        DirectionalLight {
            shadows_enabled: true,
            illuminance: lux::RAW_SUNLIGHT, // Полная освещенность от солнечного света
            ..default()
        },
        // Начальная позиция не имеет значения для вращения DirectionalLight
        Transform::default(),
        cascade_shadow_config,
    )).id();

    // Спавним сущность с TimedSkyCenter и базовыми компонентами, необходимыми для SkyCenter
    commands.spawn((
        TimedSkyCenter {
            sun: sun_id,
            planet_tilt_degrees: 23.5, // Начальное значение
            day_duration_secs: 20.0,   // Начальное значение
            night_duration_secs: 10.0,  // Начальное значение
        },
        // Добавляем обязательные компоненты для SkyCenter, даже если он не присутствует изначально.
        // Система UI добавит SkyCenter позже.
        Transform::default(),
        Visibility::Visible,
        // Добавляем StarSpawner здесь
        StarSpawner {
            star_count: 1000,
            spawn_radius: 5000.0, // Звезды должны быть очень далеко
        },
        // Опционально: добавьте маркерный компонент или имя, если у вас есть несколько таких сущностей
        // MySkyEntity,
    ));

    let sphere_mesh = meshes.add(Mesh::from(Sphere { radius: 1.0 }));

    // Сферы для пробников освещения (используем Mesh3dBundle для удобства)
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


    // Террейн (используем SceneBundle для удобства)
    commands.spawn((
        SceneRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset("terrain.glb"))),
        Transform::from_xyz(-1.0, 0.0, -0.5)
            .with_scale(Vec3::splat(0.5))
            .with_rotation(Quat::from_rotation_y(PI / 2.0)),
    ));

    // Добавляем маркерную сферу в начале координат
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(0.02))),
        MeshMaterial3d(materials.add(Color::srgb(1.0, 0.0, 0.0))),
    ));
}


// --- Система UI ---
fn ui_system(
    mut contexts: EguiContexts,
    mut commands: Commands, // Нужны команды для удаления/вставки компонентов
    // Запрашиваем сущность, которая может иметь TimedSkyCenter или SkyCenter
    // Используем Option<&mut SkyCenter>, чтобы иметь возможность изменять его, если он присутствует
    mut q_sky_entity: Query<(Entity, &mut TimedSkyCenter, Option<&mut SkyCenter>)>,
    // Запрашиваем трансформацию сущности солнца отдельно, так как она нужна внутри вложенного блока
    q_sun_transform: Query<&Transform, Without<SkyCenter>>,
) {
    // Используем get_single_mut(), который обрабатывает случай, когда запрос пуст или имеет несколько результатов
    let (entity, mut timed_config, sky_center_option) = match q_sky_entity.get_single_mut() {
        Ok(data) => data,
        Err(_) => return, // Выходим, если сущность не найдена или не уникальна
    };

    egui::Window::new("Sky Cycle Settings").show(contexts.ctx_mut(), |ui| {
        ui.heading("Timed Sky Center Settings");
        ui.label("Configure desired day/night durations. Applies to Summer Solstice.");

        ui.add(egui::Slider::new(&mut timed_config.planet_tilt_degrees, 0.0..=90.0).text("Planet Tilt (°)"));
        ui.add(egui::Slider::new(&mut timed_config.day_duration_secs, 0.1..=120.0).text("Desired Day Duration (s)")); // Предотвращаем 0 длительность в слайдере
        ui.add(egui::Slider::new(&mut timed_config.night_duration_secs, 0.1..=120.0).text("Desired Night Duration (s)")); // Предотвращаем 0 длительность в слайдере

        ui.separator();

        // Рассчитываем *потенциальные* результирующие параметры на основе текущих значений TimedSkyCenter
        let calculated_params = calculate_timed_sky_center_params(
            timed_config.planet_tilt_degrees,
            timed_config.day_duration_secs,
            timed_config.night_duration_secs,
        );

        ui.heading("Calculated Parameters (Summer Solstice)");
        if let Some((lat, year)) = calculated_params {
             ui.label(format!("Required Latitude: {:.2}°", lat));
             ui.label(format!("Required Year Fraction: {:.2}", year));
             ui.label(format!("Total Cycle Duration: {:.2} s", timed_config.day_duration_secs + timed_config.night_duration_secs));

             if ui.button("Apply Timed Settings").clicked() {
                 let total_duration = timed_config.day_duration_secs + timed_config.night_duration_secs;
                 let new_sky_center = SkyCenter {
                     latitude_degrees: lat,
                     planet_tilt_degrees: timed_config.planet_tilt_degrees,
                     year_fraction: year,
                     cycle_duration_secs: total_duration,
                     sun: timed_config.sun,
                     current_cycle_time: 0.0, // Сбрасываем время на полночь при применении
                 };

                 // Удаляем старый SkyCenter, если он существует
                 commands.entity(entity).remove::<SkyCenter>();
                 // Вставляем новый SkyCenter
                 commands.entity(entity).insert(new_sky_center);

                 info!("Applied new SkyCenter settings: Lat {:.2}°, Year Frac {:.2}, Cycle {:.2}s", lat, year, total_duration);
             }
        } else {
             ui.label("Cannot calculate parameters for these durations and tilt.");
             // Предоставляем подсказки, почему расчет не удался
             let total = timed_config.day_duration_secs + timed_config.night_duration_secs;
             if total <= 0.0 {
                 ui.label("Error: Total duration must be positive.");
             } else if timed_config.planet_tilt_degrees.abs() < f32::EPSILON && (timed_config.day_duration_secs / total - 0.5).abs() > f32::EPSILON {
                  ui.label("Error: With 0 tilt, day/night must be equal duration.");
             } else {
                 // Проверяем условия вечного дня/ночи, для которых calculate_timed_sky_center_params может вернуть None
                  if timed_config.day_duration_secs.abs() < f32::EPSILON && timed_config.night_duration_secs > 0.0 {
                     ui.label("Error: Perpetual night requires polar setup (not handled by calculation).");
                  } else if timed_config.night_duration_secs.abs() < f32::EPSILON && timed_config.day_duration_secs > 0.0 {
                     ui.label("Error: Perpetual day requires polar setup (not handled by calculation).");
                  } else {
                      ui.label("Check day/night durations or tilt; impossible combination for Summer Solstice.");
                  }
             }
        }

        ui.separator();

        // --- Отображаем информацию из активного SkyCenter (если присутствует) ---
        ui.heading("Current Active Sky Center Info");
        // Этот блок содержит всю информацию о *текущем активном* SkyCenter
        if let Some(mut sky_center) = sky_center_option { // Примечание: здесь нужен mut, чтобы разрешить изменения слайдеров

            ui.label(format!("Actual Latitude: {:.2}°", sky_center.latitude_degrees));
            ui.label(format!("Actual Planet Tilt: {:.2}°", sky_center.planet_tilt_degrees));
            ui.label(format!("Actual Year Fraction: {:.2}", sky_center.year_fraction));
            ui.add(egui::Slider::new(&mut sky_center.cycle_duration_secs, 1.0..=120.0).text("Actual Cycle Duration (s)")); // Позволяем изменять фактическую длительность цикла

            // Опция паузы/воспроизведения времени
            let is_paused = sky_center.cycle_duration_secs <= 0.0;
            if ui.button(if is_paused { "Play" } else { "Pause" }).clicked() {
                 if is_paused {
                      // Восстанавливаем значение длительности, если на паузе
                      sky_center.cycle_duration_secs = 30.0; // Восстанавливаем цикл на 30 секунд
                       // Убеждаемся, что current_cycle_time находится в пределах после снятия паузы
                      sky_center.current_cycle_time %= sky_center.cycle_duration_secs.max(f32::EPSILON); // max(f32::EPSILON) для предотвращения деления на ноль
                 } else {
                      sky_center.cycle_duration_secs = 0.0; // Пауза
                 }
            }

            // Слайдер времени для активного цикла
            let mut current_cycle_time = sky_center.current_cycle_time;
            if sky_center.cycle_duration_secs > 0.0 {
                if ui.add(egui::Slider::new(&mut current_cycle_time, 0.0..=sky_center.cycle_duration_secs).text("Current Cycle Time (s)")).changed() {
                   sky_center.current_cycle_time = current_cycle_time;
                }
            } else {
                 // Если на паузе, позволяем пользователю установить время через долю 0-1
                 let mut current_cycle_fraction = sky_center.current_cycle_time; // Используем current_cycle_time как долю 0-1
                 if ui.add(egui::Slider::new(&mut current_cycle_fraction, 0.0..=1.0).text("Current Cycle Fraction (0-1)")).changed() {
                    sky_center.current_cycle_time = current_cycle_fraction.clamp(0.0, 1.0);
                 }
                 ui.label("Time is paused.");
            }


            // Получаем текущую информацию о солнце из его трансформации
            ui.separator();
            ui.heading("Current Sun Info");
            // Этот вложенный блок содержит информацию о солнце и график, зависящие от наличия трансформации солнца
            // Используем отдельный запрос для трансформации солнца
            let sun_transform_actual = q_sun_transform.get(sky_center.sun).ok();


            if let Some(sun_transform) = sun_transform_actual { // Начало блока sun_transform_actual

                 // Вектор ОТ наблюдателя К солнцу - это Transform.local_z(), если +Z света указывает на солнце.
                 let current_sun_direction = sun_transform.local_z();

                 let elevation_rad = current_sun_direction.y.asin(); // Y - вверх
                 let elevation_degrees = elevation_rad * RADIANS_TO_DEGREES;
                 ui.label(format!("Sun Elevation: {:.1}°", elevation_degrees));

                 // X - Восток, Z - Север. Угол от +Z к +X.
                 let horizontal_direction = Vec2::new(current_sun_direction.x, current_sun_direction.z);
                 let heading_rad = horizontal_direction.x.atan2(horizontal_direction.y); // atan2(East, North)
                 let mut heading_degrees = heading_rad * RADIANS_TO_DEGREES;
                 if heading_degrees < 0.0 { heading_degrees += 360.0; } // Нормализуем 0-360
                  ui.label(format!("Sun Heading (from North): {:.1}°", heading_degrees));

                  let hour_fraction = if sky_center.cycle_duration_secs > 0.0 {
                      sky_center.current_cycle_time / sky_center.cycle_duration_secs
                  } else {
                      sky_center.current_cycle_time.clamp(0.0, 1.0) // Используем значение 0-1 напрямую, если на паузе
                  };
                  let hour_of_day = hour_fraction * 24.0;
                  ui.label(format!("Time of Day: {:.2} hours", hour_of_day));

                  ui.separator();
                  ui.heading("Sun Trajectory Plot (Active Settings)");

                  let n_points = 100;
                  let latitude_rad = sky_center.latitude_degrees * DEGREES_TO_RADIANS;
                  let axial_tilt_rad = sky_center.planet_tilt_degrees * DEGREES_TO_RADIANS;
                  let year_fraction = sky_center.year_fraction; // Используем фактическую долю года из SkyCenter

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
                      let heading_degrees = heading_rad * RADIANS_TO_DEGREES;
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

            } else { // else для sun_transform_actual
                ui.label("Sun entity transform not found.");
            } // Конец блока sun_transform_actual

        } else { // else для sky_center_option
            ui.label("SkyCenter component not active yet. Apply settings first.");
        } // Конец блока sky_center_option

    }); // Конец замыкания Window
} // Конец функции ui_system