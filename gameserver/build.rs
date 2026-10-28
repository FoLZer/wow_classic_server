use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=migrations");
    println!("cargo::rustc-check-cfg=cfg(internal_migrations)");
    if Path::new("internal/migrations").exists() {
        println!("cargo:rerun-if-changed=internal/migrations");
        println!("cargo:rustc-cfg=internal_migrations");
    }
}
