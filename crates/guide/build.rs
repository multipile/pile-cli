// Every file in pages/, included by name, so the pages the binary carries are
// the files there, and nothing lists them (src/lib.rs).
use std::path::Path;

fn main() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("pages");
    println!("cargo:rerun-if-changed={}", dir.display());
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .expect("crates/guide/pages is there")
        .map(|e| e.expect("a readable entry").path())
        .collect();
    files.sort();
    let mut out = String::from("&[\n");
    for path in files {
        println!("cargo:rerun-if-changed={}", path.display());
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        out.push_str(&format!(
            "    ({name:?}, include_str!({:?})),\n",
            path.display().to_string()
        ));
    }
    out.push_str("]\n");
    let to = Path::new(&std::env::var("OUT_DIR").unwrap()).join("pages.rs");
    std::fs::write(to, out).expect("OUT_DIR is writable");
}
