use std::process::Command;

fn main() {
    let git_commit = build_data::get_git_commit().unwrap_or_default();
    let git_short = build_data::get_git_commit_short().unwrap_or_else(|_| "unknown".to_string());
    let build_date = build_data::format_date(build_data::now());

    // Run `npm run build` to build the Tailwind CSS
    let status = Command::new("npm")
        .args(["run", "build"])
        .status()
        .expect("failed to build Tailwind CSS");
    if !status.success() {
        panic!("failed to build Tailwind CSS");
    }

    // Run `npm run copy` to copy the htmlx source
    let status = Command::new("npm")
        .args(["run", "copy"])
        .status()
        .expect("failed to copy HTMX");
    if !status.success() {
        panic!("failed to copy HTMX");
    }

    println!("cargo:rerun-if-changed=templates");
    println!("cargo:rerun-if-changed=style");
    println!("cargo:rerun-if-changed=postcss.config.js");
    println!("cargo:rerun-if-changed=tailwind.config.js");

    println!("cargo:rustc-env=CARGO_BUILD_DATE={}", build_date);
    println!("cargo:rustc-env=CARGO_GIT_COMMIT={git_commit}");
    println!("cargo:rustc-env=CARGO_GIT_SHORT={git_short}");
}
