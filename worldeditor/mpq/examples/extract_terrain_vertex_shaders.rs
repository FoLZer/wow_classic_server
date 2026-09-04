use std::{fs, path::PathBuf};

use wow_mpq::PatchChain;

const SHADERS: [&str; 16] = [
    "terrain.bls",
    "terrain_s.bls",
    "terrain_u.bls",
    "terrain_us.bls",
    "terrain1.bls",
    "terrain1_s.bls",
    "terrain2.bls",
    "terrain2_s.bls",
    "terrain3.bls",
    "terrain3_s.bls",
    "terrain4.bls",
    "terrain4_s.bls",
    "terrainp.bls",
    "terrainp_s.bls",
    "terrainp_u.bls",
    "terrainp_us.bls",
];

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let data = PathBuf::from("worldeditor/Data");
    let output = PathBuf::from("worldeditor/tmp/terrain_shaders/vertex");
    fs::create_dir_all(&output)?;

    let chain = PatchChain::from_archives_parallel(vec![
        (data.join("patch-2.MPQ"), 101),
        (data.join("patch.MPQ"), 100),
        (data.join("misc.MPQ"), 2),
        (data.join("base.MPQ"), 0),
    ])?;
    for filename in SHADERS {
        let path = format!("Shaders\\Vertex\\{filename}");
        if let Ok(bytes) = chain.read_file_concurrent(&path) {
            fs::write(output.join(filename), &bytes)?;
            println!("{path}: {} bytes", bytes.len());
        }
    }

    Ok(())
}