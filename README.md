
![exact_location](https://github.com/user-attachments/assets/bb9b08fe-beb6-47d3-b544-576fe5c83156)


# bevy_sun_move

A Bevy plugin for simulating realistic sun movement, integrating well with Bevy's `Atmosphere` and PBR lighting.

This plugin provides components and systems to:

1.  **Calculate necessary astronomical parameters** (latitude, year fraction) to achieve a *desired* day length, night length, and maximum sun height (noon altitude). This allows very fast to specify game timings with saving correct sun move on the sky.
2.  **Animate a directional light (the "sun")** across the sky based on astronomical principles (latitude, year fraction, time of day).
3.  **Rotate a "sky sphere" entity** with the apparent motion of the celestial sphere, useful for attaching other celestial bodies or a sky mesh.
4.  **Include an optional basic random star field** whose visibility can fade during the day.

## Installation

Add `bevy_sun_move` to your `Cargo.toml`:

```bash
cargo add bevy_sun_move
```

# Usage

Add the plugin

```rust
use bevy::prelude::*;
use bevy_sun_move::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(SunMovePlugin)
        .add_plugins(RandomStarsPlugin) // Optional for very simple stars on night sky
        .run();
}
```

Then

1. Spawn a DirectionalLight: This will be your sun. It needs a Transform, but the SunMovePlugin will manage its position and rotation.
2. Spawn a SkyCenter entity: This entity holds the configuration and state for the sky simulation. 

You have two main ways to configure the SkyCenter:

**Using TimedSkyConfig (Recommended):** Define your desired day/night lengths and maximum sun height. The plugin will calculate the necessary astronomical parameters for correct sun move.

```rust
// In your setup system:
fn setup_scene(mut commands: Commands, ...) {
    let sun_id = commands
        .spawn((
            DirectionalLight {
                shadows_enabled: true,
                illuminance: light_consts::lux::RAW_SUNLIGHT, // Adjust illuminance as needed
                ..default()
            },
            Transform::default(), // Transform will be updated by the plugin
        ))
        .id();

    let timed_sky_config = TimedSkyConfig {
        sun_entity: sun_id,
        day_duration_secs: 10.0,    // 10 seconds of daylight
        night_duration_secs: 5.0,   // 5 seconds of nighttime (15s total cycle)
        max_sun_height_deg: 60.0, // Sun reaches 60 degrees altitude at noon
        planet_tilt_degrees: 23.5,  // Earth's tilt (default)
        ..default()
    };

    // Calculate and spawn the SkyCenter
    if let Some(sky_center) = SkyCenter::from_timed_config(&timed_sky_config) {
        commands.spawn((
            sky_center,
            // Optional: Add StarSpawner if you want the built-in stars
            StarSpawner {
                star_count: 1000,
                spawn_radius: 5000.0, // Stars distance
            },
        ));
    } else {
        // Handle case where calculation failed (e.g., impossible parameters)
        error!("Failed to create SkyCenter from timed config.");
    }

    // ... rest of your scene setup
}
```

**Directly using SkyCenter:** If you want the specific latitude and year fraction (e.g., simulating a real-world location and date), you can set them directly.

```rust
// In your setup system:
fn setup_scene(mut commands: Commands, ...) {
     let sun_id = commands
        .spawn((
            DirectionalLight {
                shadows_enabled: true,
                illuminance: light_consts::lux::RAW_SUNLIGHT, // Adjust illuminance as needed
                ..default()
            },
            Transform::default(), // Transform will be updated by the plugin
        ))
        .id();

     commands.spawn((
         SkyCenter {
             sun: sun_id,
             latitude_degrees: 51.5,    // e.g., London's approximate latitude
             planet_tilt_degrees: 23.5, // Earth's axial tilt
             year_fraction: 0.25,       // e.g., Summer Solstice
             cycle_duration_secs: 60.0, // 60-second day/night cycle
             current_cycle_time: 0.0,   // Start at midnight
             ..default()
         },
         Visibility::Visible,
         Transform::default(),
         StarSpawner { star_count: 1000, spawn_radius: 5000.0 }, // Optional
     ));

     // ... rest of your scene setup
 }
```

For better results **add Atmosphere to your Camera** with same way as bevy example describe 
```rust
use bevy::{
    core_pipeline::{bloom::Bloom, tonemapping::Tonemapping},
    pbr::{Atmosphere, AtmosphereSettings},
    prelude::*,
    render::camera::Exposure,
};

// In your setup system:
fn setup_scene(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        // ... camera transform
        Camera { hdr: true, ..default() },
        Atmosphere::EARTH, // Or customize with AtmosphereSettings
        AtmosphereSettings {
            aerial_view_lut_max_distance: 3.2e5,
            scene_units_to_m: 1e+4, // Scale your scene units to meters
            ..Default::default()
        },
        Exposure::SUNLIGHT, // Recommended for good dynamic range
        Tonemapping::AcesFitted, // Recommended tonemapping
        Bloom::NATURAL, // Optional post-processing
    ));

    // ... rest of your scene setup
}
```

The update_sky_center system will automatically run in the Update schedule, advancing current_cycle_time and updating the sun's transform based on the SkyCenter parameters.

# Components and Resources
`SkyCenter`
A component added to an entity (often the root of your scene or a dedicated sky entity) that defines the parameters of the sky simulation.
- latitude_degrees: Observer's latitude in degrees (-90 to 90).
- planet_tilt_degrees: Axial tilt of the planet in degrees.
- year_fraction: Fraction of the year (0.0 to 1.0), where 0.0 is Vernal Equinox, 0.25 is Summer Solstice, 0.5 is Autumnal Equinox, 0.75 is Winter Solstice (for positive tilt).
- cycle_duration_secs: Total duration of a full day/night cycle in seconds.
- sun: The Entity ID of the DirectionalLight to control.
- current_cycle_time: The current time within the cycle_duration_secs (0.0 to cycle_duration_secs). Can be modified to pause or set the time.

`TimedSkyConfig`

A temporary struct used to calculate SkyCenter parameters based on desired timings.
- planet_tilt_degrees: Axial tilt of the planet in degrees.
- day_duration_secs: Desired duration of daylight (sun above horizon) in seconds.
- night_duration_secs: Desired duration of nighttime (sun below horizon) in seconds.
- max_sun_height_deg: Desired maximum sun height (altitude) in degrees during the day.
- sun_entity: The Entity ID of the DirectionalLight.
Used with `SkyCenter::from_timed_config(&timed_config) -> Option<SkyCenter>`. The function returns `None` if the requested timings and max height are impossible for the given tilt (e.g., requesting 24-hour day at the equator with 0 tilt, or a max height greater than 90 degrees).

# Contributing

Contributions are welcome! Feel free to open issues or pull requests on the GitHub repository.

# License

This project is licensed under the MIT license (or similar, specify your license choice).


