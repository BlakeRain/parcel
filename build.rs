use std::process::Command;

fn main() {
    let build_date = build_data::format_date(build_data::now());
    let profile = std::env::var("PROFILE").expect("PROFILE");
    let is_debug = profile == "debug";

    // Run `npm install` to install the npm packages
    let status = Command::new("npm")
        .args(["install"])
        .status()
        .expect("to install npm packages");
    if !status.success() {
        panic!("failed to install npm packages");
    }

    // Run `npm run build` to build the Tailwind CSS
    let status = Command::new("npm")
        .args(["run", if is_debug { "build-dev" } else { "build" }])
        .status()
        .expect("failed to build Tailwind CSS");
    if !status.success() {
        panic!("failed to build Tailwind CSS");
    }

    // Run `npm run copy` to copy the htmlx and _hyperscript source
    let status = Command::new("npm")
        .args(["run", if is_debug { "copy-dev" } else { "copy" }])
        .status()
        .expect("failed to copy HTMX");
    if !status.success() {
        panic!("failed to copy HTMX");
    }

    println!("cargo:rerun-if-changed=templates");
    println!("cargo:rerun-if-changed=style");
    println!("cargo:rerun-if-changed=package.json");
    println!("cargo:rerun-if-changed=postcss.config.js");
    println!("cargo:rerun-if-changed=tailwind.config.js");

    println!("cargo:rustc-env=CARGO_PROFILE={profile}");
    println!("cargo:rustc-env=CARGO_BUILD_DATE={build_date}");
}
