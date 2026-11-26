//! Embeds every level file under `levels/` into the binary, so adding a level
//! only needs the file and its entry in `levels/worlds.ron`; no Rust code.
//! Generates `$OUT_DIR/level_files.rs` with a `LEVEL_FILES` table of
//! `(file stem, contents)`, which `campaign.rs` includes.

use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

fn main() {
    let levels = Path::new(env!("CARGO_MANIFEST_DIR")).join("levels");
    println!("cargo:rerun-if-changed={}", levels.display());

    let mut stems: Vec<(String, String)> = fs::read_dir(&levels)
        .expect("levels/ exists")
        .map(|entry| entry.expect("readable entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "ron"))
        .filter(|path| path.file_stem().is_some_and(|stem| stem != "worlds"))
        .map(|path| {
            println!("cargo:rerun-if-changed={}", path.display());
            let stem = path.file_stem().unwrap().to_string_lossy().into_owned();
            (stem, path.display().to_string())
        })
        .collect();
    stems.sort();

    let mut code = String::from("pub const LEVEL_FILES: &[(&str, &str)] = &[\n");
    for (stem, path) in stems {
        writeln!(code, "    ({stem:?}, include_str!({path:?})),").unwrap();
    }
    code.push_str("];\n");

    let out = Path::new(&env::var("OUT_DIR").expect("set by cargo")).join("level_files.rs");
    fs::write(out, code).expect("OUT_DIR is writable");
}
