// What `pile version` says about the compiler. Only a build script can ask
// it: the version of the rustc doing the build, and the target it builds for.
use std::env;
use std::process::Command;

fn main() {
    let rustc = env::var("RUSTC").unwrap_or_else(|_| "rustc".into());
    let version = Command::new(&rustc)
        .arg("--version")
        .output()
        .ok()
        .filter(|out| out.status.success())
        .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_string())
        .unwrap_or_else(|| "rustc (unknown)".into());
    let target = env::var("TARGET").unwrap_or_else(|_| "unknown".into());

    println!("cargo:rustc-env=PILE_RUSTC={version}");
    println!("cargo:rustc-env=PILE_TARGET={target}");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=RUSTC");
    println!("cargo:rerun-if-env-changed=PILE_VERSION");
    println!("cargo:rerun-if-env-changed=PILE_COMMIT");
    println!("cargo:rerun-if-env-changed=PILE_DATE");
}
