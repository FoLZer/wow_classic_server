mod combined_alpha_map;
mod liquid_material;
mod map_loader;
mod render_controls;
mod terrain_material;

use std::{path::PathBuf, str::FromStr, sync::Arc};

#[cfg(feature = "realistic-lighting")]
use bevy::light::CascadeShadowConfigBuilder;
use bevy::{
    diagnostic::{EntityCountDiagnosticsPlugin, FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    pbr::ExtendedMaterial,
    prelude::*,
};
use bevy_camera_controller::free_camera::{FreeCamera, FreeCameraPlugin};
use serde::{Deserialize, Serialize};
use wow_mpq::PatchChain;

use crate::{
    liquid_material::LiquidMaterial,
    map_loader::{TerrainEditorPlugin, animate_objects, load_map, stream_terrain_chunks},
    render_controls::{
        RenderSettings, apply_render_visibility, setup_render_controls, update_render_controls,
        update_slider_visuals,
    },
    terrain_material::TerrainMaterial,
};

#[derive(Deserialize, Serialize, Resource)]
#[serde(default)]
struct AppSettings {
    mpq_directory_path: PathBuf,
    terrain_view_distance: f32,
    object_view_distance: f32,
    ground_effect_view_distance: f32,
    log_diagnostics: bool,
    focus_wmo_camera_on_start: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            mpq_directory_path: PathBuf::from_str("./Data").unwrap(),
            terrain_view_distance: 50_000.0,
            object_view_distance: 3_000.0,
            ground_effect_view_distance: 40.0,
            log_diagnostics: false,
            focus_wmo_camera_on_start: true,
        }
    }
}

fn main() {
    let config: AppSettings = confy::load_path("./worldeditor_config.toml").unwrap();
    let render_settings = RenderSettings {
        render_adts: true,
        render_objects: true,
        render_ground_effects: true,
        adt_distance: config.terrain_view_distance,
        object_distance: config.object_view_distance,
        ground_effect_distance: config.ground_effect_view_distance,
        edit_mode: Default::default(),
    };

    let mpqs = PatchChain::from_archives_parallel(vec![
        (config.mpq_directory_path.join("patch-2.MPQ"), 101),
        (config.mpq_directory_path.join("patch.MPQ"), 100),
        (config.mpq_directory_path.join("interface.MPQ"), 9),
        (config.mpq_directory_path.join("wmo.MPQ"), 8),
        (config.mpq_directory_path.join("texture.MPQ"), 7),
        (config.mpq_directory_path.join("terrain.MPQ"), 6),
        (config.mpq_directory_path.join("speech.MPQ"), 5),
        (config.mpq_directory_path.join("sound.MPQ"), 4),
        (config.mpq_directory_path.join("model.MPQ"), 3),
        (config.mpq_directory_path.join("misc.MPQ"), 2),
        (config.mpq_directory_path.join("dbc.MPQ"), 1),
        (config.mpq_directory_path.join("base.MPQ"), 0),
    ])
    .unwrap();

    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins,
        MaterialPlugin::<ExtendedMaterial<StandardMaterial, TerrainMaterial>>::default(),
        MaterialPlugin::<ExtendedMaterial<StandardMaterial, LiquidMaterial>>::default(),
    ));
    if config.log_diagnostics {
        app.add_plugins((
            FrameTimeDiagnosticsPlugin::default(),
            EntityCountDiagnosticsPlugin::default(),
            LogDiagnosticsPlugin::default(),
        ));
    }
    app.add_plugins((FreeCameraPlugin, MeshPickingPlugin, TerrainEditorPlugin))
        //.add_plugins(EguiPlugin::default())
        //.add_plugins(WorldInspectorPlugin::new())
        .insert_resource(MPQResource {
            mpqs: Arc::new(mpqs),
        })
        .insert_resource(render_settings)
        .insert_resource(config)
        .add_systems(Startup, (setup, setup_render_controls))
        .add_systems(
            Update,
            (
                update_render_controls,
                update_slider_visuals,
                apply_render_visibility.after(update_render_controls),
                stream_terrain_chunks.after(update_render_controls),
                animate_objects.after(stream_terrain_chunks),
            ),
        )
        .run();
}

#[derive(Resource)]
struct MPQResource {
    pub mpqs: Arc<PatchChain>,
}

fn setup(mut commands: Commands, mpqs_res: Res<MPQResource>, settings: Res<AppSettings>) {
    let camera = commands
        .spawn((
            Camera3d::default(),
            Projection::Perspective(PerspectiveProjection {
                near: 0.1,
                far: 75_000.0,
                ..Default::default()
            }),
            Transform::from_xyz(0.0, 32_000.0, 0.0).looking_at(Vec3::ZERO, Vec3::Z),
            FreeCamera {
                walk_speed: 50.0,
                run_speed: 600.0,
                ..Default::default()
            },
            Msaa::Off,
        ))
        .id();

    #[cfg(feature = "realistic-lighting")]
    {
        commands.entity(camera).insert(AmbientLight {
            color: Color::srgb(0.58, 0.68, 0.82),
            brightness: 90.0,
            ..default()
        });
        commands.spawn((
            Name::new("Realistic daylight"),
            DirectionalLight {
                color: Color::srgb(1.0, 0.91, 0.76),
                illuminance: 18_000.0,
                shadow_maps_enabled: true,
                ..default()
            },
            CascadeShadowConfigBuilder {
                first_cascade_far_bound: 150.0,
                maximum_distance: settings.object_view_distance + 500.0,
                ..default()
            }
            .build(),
            Transform::default().looking_to(Vec3::new(-0.7, -1.0, -0.45), Vec3::Y),
        ));
    }

    let wmo_camera_transform = load_map(&mpqs_res.mpqs, &mut commands, 0);
    if settings.focus_wmo_camera_on_start
        && let Some(wmo_camera_transform) = wmo_camera_transform
    {
        commands.entity(camera).insert(wmo_camera_transform);
    }
}

// Some file names appear to be uppercased inside mpqs, this function tries to handle this case
pub fn mpq_read_file(mpqs: &PatchChain, filepath: &str) -> Result<Vec<u8>, wow_mpq::Error> {
    match mpqs.read_file_concurrent(filepath) {
        Ok(v) => Ok(v),
        Err(wow_mpq::Error::FileNotFound(_)) => {
            let filepath = {
                if let Some((left, right)) = filepath.rsplit_once("\\") {
                    // Only uppercase the filename, without the extension if present
                    let right = if let Some((left, right)) = right.rsplit_once('.') {
                        format!("{}.{right}", left.to_uppercase())
                    } else {
                        right.to_uppercase()
                    };
                    format!("{left}\\{}", right)
                } else {
                    filepath.to_uppercase()
                }
            };
            mpqs.read_file_concurrent(&filepath)
        }
        Err(e) => Err(e),
    }
}
