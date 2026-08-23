use std::{
    io::{BufRead, BufReader, Cursor},
    path::{Path, PathBuf},
    sync::Arc,
};

use convert_case::{Case, Casing};
use dbd_reader::{Build, DBCType, DBDefinition};
use lazy_static::lazy_static;
use reqwest::Client;

async fn download_def_file(file_name: &str) -> String {
    const BASE_URL: &str = "https://raw.githubusercontent.com/wowdev/WoWDBDefs/master/definitions/";
    let r = CLIENT
        .get(format!("{}{}.dbd", BASE_URL, file_name))
        .send()
        .await
        .unwrap();
    r.text().await.unwrap()
}

async fn create_data_def_file(
    out_dir: &Path,
    file_name: &str,
    build: &Build,
    skip_not_found: bool,
) -> bool {
    let def = DBDefinition::from_lines(
        &mut BufReader::new(Cursor::new(&download_def_file(file_name).await)).lines(),
    )
    .unwrap();
    let current_build_def = {
        let mut current_build_def = None;
        'out: for def in &def.version_definitions {
            for build_range in &def.build_ranges {
                if build_range.contains(build) {
                    current_build_def = Some(def);
                    break 'out;
                }
            }
            for b in &def.builds {
                if b.eq(build) {
                    current_build_def = Some(def);
                    break 'out;
                }
            }
        }
        if skip_not_found && current_build_def.is_none() {
            return false;
        }
        current_build_def.unwrap_or_else(|| panic!("Not found version for definition: {:?}", def))
    };
    let mut path = out_dir.join(file_name);
    path.set_extension("rs");
    std::fs::write(
        path,
        format!(
            "
#[allow(unused)]
#[derive(macros::Record, Debug)]
pub struct {} {{
{}
}}
    ",
            file_name.to_case(Case::UpperCamel),
            current_build_def
                .definitions
                .iter()
                .map(|d| {
                    format!(
                        "\tpub {}: {},",
                        d.name.to_case(Case::Snake).replace("type", "ty"),
                        {
                            let ty = &def.column_definitions.get(&d.name).unwrap().ty;
                            let rust_ty = match ty {
                                DBCType::Uint => match d.size {
                                    8 => "u8",
                                    16 => "u16",
                                    32 => "u32",
                                    64 => "u64",
                                    _ => {
                                        panic!("Unexpected size: {}", d.size)
                                    }
                                },
                                DBCType::Int => match d.size {
                                    8 => "i8",
                                    16 => "i16",
                                    32 => "i32",
                                    64 => "i64",
                                    _ => {
                                        panic!("Unexpected size: {}", d.size)
                                    }
                                },
                                DBCType::Float => "f32",
                                //DBCType::String => "dbc_reader::structs::StringIndex",
                                DBCType::String => "std::ffi::CString",
                                DBCType::LocString => {
                                    if build.expansion >= 4 {
                                        //Cataclysm removed LocString
                                        "dbc_reader::structs::StringIndex"
                                    } else if build.expansion >= 2
                                        && build.major >= 1
                                        && build.build >= 6692
                                    {
                                        "dbc_reader::structs::LocString<16>"
                                    } else {
                                        "dbc_reader::structs::LocString<8>"
                                    }
                                }
                            };
                            if d.arr_length != 0 {
                                format!("[{}; {}]", rust_ty, d.arr_length)
                            } else {
                                rust_ty.to_owned()
                            }
                        }
                    )
                })
                .collect::<Vec<_>>()
                .join("\n")
        ),
    )
    .unwrap();
    true
}

fn build_includes_file(path: &Path, file_names: &[&str]) {
    let mut s = String::new();
    s.push_str("#[allow(unused)]\nuse std::ffi::CString;\n");
    for file_name in file_names {
        s.push_str(&format!(
            "include!(concat!(env!(\"OUT_DIR\"), \"/defs/\", \"{}.rs\"));\n",
            file_name
        ));
    }
    std::fs::write(path, s).unwrap();
}

async fn creates(out_dir: &Path, file_names: &'static [&'static str], build: &'static Build) {
    let data_dir = out_dir.join("defs");
    if !data_dir.is_dir() {
        std::fs::create_dir(&data_dir).unwrap();
    }
    let added_files = Arc::new(tokio::sync::Mutex::new(Vec::new()));
    let mut tasks = Vec::new();
    for file_name in file_names {
        let data_dir = data_dir.clone();
        let added_files = added_files.clone();
        tasks.push(tokio::task::spawn(async move {
            if create_data_def_file(&data_dir, file_name, build, true).await {
                added_files.lock().await.push(*file_name)
            }
        }));
    }
    futures::future::join_all(tasks).await;

    build_includes_file(
        &out_dir.join("includes.rs"),
        Arc::into_inner(added_files)
            .unwrap()
            .into_inner()
            .as_slice(),
    );
}

lazy_static! {
    static ref CLIENT: Client = Client::new();
}

#[tokio::main]
async fn main() {
    //println!("cargo:rerun-if-changed=build.rs");

    let out_dir = PathBuf::from(&std::env::var_os("OUT_DIR").unwrap());

    creates(
        &out_dir,
        &[
            "AreaTable",
            "GroundEffectDoodad",
            "GroundEffectTexture",
            "Map",
        ],
        &Build {
            expansion: 1,
            major: 12,
            minor: 1,
            build: 5875,
        },
    )
    .await;
}
