mod map_loader;
mod terrain_material;

use std::{path::PathBuf, str::FromStr};

use bevy::{camera::visibility::VisibilityRange, pbr::ExtendedMaterial, prelude::*, render::view::RenderVisibilityRangePlugin};
use bevy_camera_controller::free_camera::{FreeCamera, FreeCameraPlugin};
use serde::{Deserialize, Serialize};
use wow_mpq::PatchChain;

use crate::{map_loader::load_map, terrain_material::TerrainMaterial};

#[derive(Deserialize, Serialize)]
struct AppSettings {
    mpq_directory_path: PathBuf,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            mpq_directory_path: PathBuf::from_str("./Data").unwrap(),
        }
    }
}

fn main() {
    let config: AppSettings = confy::load_path("./worldeditor_config.toml").unwrap();

    let mpqs = PatchChain::from_archives_parallel(vec![
        (config.mpq_directory_path.join("patch-2.MPQ"), 101),
        (config.mpq_directory_path.join("patch.MPQ"), 100),
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

    App::new()
        .add_plugins((
            DefaultPlugins,
            MaterialPlugin::<ExtendedMaterial<StandardMaterial, TerrainMaterial>>::default(),
        ))
        .add_plugins(FreeCameraPlugin)
        //.add_plugins(EguiPlugin::default())
        //.add_plugins(WorldInspectorPlugin::new())
        .insert_resource(MPQResource { mpqs })
        .add_systems(Startup, setup)
        .run();
}

#[derive(Resource)]
struct MPQResource {
    pub mpqs: PatchChain,
}

fn setup(
    mut commands: Commands,
    terrain_materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, TerrainMaterial>>>,
    meshes: ResMut<Assets<Mesh>>,
    images: ResMut<Assets<Image>>,
    mut mpqs_res: ResMut<MPQResource>,
) {
    commands.spawn((
        Camera3d::default(),
        FreeCamera {
            walk_speed: 50.0,
            run_speed: 600.0,
            ..Default::default()
        },
        Msaa::Off,
    ));

    load_map(
        &mut mpqs_res.mpqs,
        commands,
        terrain_materials,
        meshes,
        images,
        1,
    );
}

// Some file names appear to be uppercased inside mpqs, this function tries to handle this case
pub fn mpq_read_file(mpqs: &mut PatchChain, filepath: &str) -> Result<Vec<u8>, wow_mpq::Error> {
    match mpqs.read_file(filepath) {
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
            mpqs.read_file(&filepath)
        }
        Err(e) => Err(e),
    }
}
