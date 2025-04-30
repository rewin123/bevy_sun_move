// Its definetely not the best way to do this, better to use a texture or some particle system
// So this is just for testing purposes


use bevy::{pbr::NotShadowCaster, prelude::*};
use rand::Rng;

use crate::SkyCenter;

pub struct RandomStarsPlugin;

impl Plugin for RandomStarsPlugin {
    fn build(&self, app: &mut App) {
        // if !app.is_plugin_added::<AutoExposurePlugin>() {
        //     app.add_plugins(AutoExposurePlugin);
        // }
        app.add_systems(Startup, setup_star_spawner);
        app.add_systems(Update, on_change_spawner);
        app.add_systems(Update, update_star_illuminance);
    }
}

#[derive(Component)]
pub struct StarSpawner {
    pub star_count: u32,
    pub spawn_radius: f32,
}

#[derive(Component)]
pub struct Star;

#[derive(Resource)]
pub struct StarSpawnerCache {
    pub mesh: Handle<Mesh>,
    pub material: Handle<StandardMaterial>,
}

fn setup_star_spawner(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mesh = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    let material = materials.add(StandardMaterial {
        base_color: Color::srgba(0.0, 0.0, 0.0, 1.0),
        alpha_mode: AlphaMode::Add,
        ..default()
    });
    commands.insert_resource(StarSpawnerCache { mesh, material });
}

fn on_change_spawner(
    mut commands: Commands,
    mut q_star_spawner: Query<(Entity, &mut StarSpawner, Option<&Children>), Changed<StarSpawner>>,
    q_star: Query<Entity, With<Star>>,
    star_spawner_cache: Res<StarSpawnerCache>,
) {
    for (entity, star_spawner, children) in q_star_spawner.iter_mut() {
        if let Some(children) = children {
            for star in children.iter() {
                if q_star.contains(star) {
                    commands.entity(star).despawn();
                }
            }
        }

        let mut rng = rand::rng();
        for _ in 0..star_spawner.star_count {
            let phi = rng.random_range(0.0..2.0 * std::f32::consts::PI);
            let theta = rng.random_range(0.0..std::f32::consts::PI);
            let x = star_spawner.spawn_radius * theta.sin() * phi.cos();
            let y = star_spawner.spawn_radius * theta.cos();
            let z = star_spawner.spawn_radius * theta.sin() * phi.sin();

            let id = commands
                .spawn((
                    Star,
                    Transform::from_xyz(x, y, z)
                        .with_scale(Vec3::ONE * star_spawner.spawn_radius / 500.0),
                    Mesh3d(star_spawner_cache.mesh.clone()),
                    MeshMaterial3d(star_spawner_cache.material.clone()),
                    NotShadowCaster,
                ))
                .id();

            commands.entity(entity).add_child(id);
        }
    }
}

fn update_star_illuminance(
    cache: Res<StarSpawnerCache>,
    q_sky_center: Query<&SkyCenter>,
    q_transforms: Query<&Transform>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Ok(sky_center) = q_sky_center.single() else {
        return;
    };

    let Ok(sun_transform) = q_transforms.get(sky_center.sun) else {
        return;
    };

    let mut sun_height = sun_transform.translation.y;

    let day_illuminance = 0.0;
    let day_point = 0.1;

    let night_illuminance = 1.0;
    let night_point = -0.1;

    sun_height = sun_height.clamp(night_point, day_point);
    sun_height = (sun_height - night_point) / (day_point - night_point);

    let illuminance = night_illuminance + sun_height * (day_illuminance - night_illuminance);

    materials.get_mut(cache.material.id()).unwrap().emissive =
        LinearRgba::rgb(illuminance, illuminance, illuminance);
}
