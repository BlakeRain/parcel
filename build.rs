use std::process::Command;

fn main() {
    let git_commit = build_data::get_git_commit_short().unwrap_or_else(|_| "unknown".to_string());
    let build_date = build_data::format_date(build_data::now());

    // Run `npm run build` to build the Tailwind CSS
    let status = Command::new("npm")
        .args(["run", "build"])
        .status()
        .expect("failed to build Tailwind CSS");
    if !status.success() {
        panic!("failed to build Tailwind CSS");
    }

    println!("cargo:rerun-if-changed=templates");
    println!("cargo:rustc-env=CARGO_BUILD_DATE={}", build_date);
    println!(
        "cargo:rustc-env=CARGO_GIT_COMMIT={}",
        if git_commit.is_empty() {
            "unknown"
        } else {
            &git_commit
        }
    );

    build_data::no_debug_rebuilds();
}
