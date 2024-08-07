use std::process::Command;

fn main() {
    let build_date = build_data::format_date(build_data::now());
    let profile = std::env::var("PROFILE").expect("PROFILE");
    let is_debug = profile == "debug";

    fn mkdir(dir: &str) {
        if std::fs::metadata(dir).is_ok() {
            return;
        }

        if let Err(err) = std::fs::create_dir_all(dir) {
            panic!("Failed to create directory {dir}: {err:?}");
        }
    }

    mkdir("static");
    mkdir("static/icons");
    mkdir("static/scripts");
    mkdir("static/scripts/bundles");
    mkdir("static/scripts/vendor");

    let copies = vec![
        (
            !is_debug,
            "node_modules/htmx.org/dist/htmx.js",
            "static/scripts/vendor/htmx.js",
        ),
        (
            is_debug,
            "node_modules/htmx.org/dist/htmx.min.js",
            "static/scripts/vendor/htmx.js",
        ),
        (
            true,
            "node_modules/lucide-static/font/lucide.css",
            "static/icons/lucide.css",
        ),
        (
            true,
            "node_modules/lucide-static/font/lucide.eot",
            "static/icons/lucide.eot",
        ),
        (
            true,
            "node_modules/lucide-static/font/lucide.woff",
            "static/icons/lucide.woff",
        ),
        (
            true,
            "node_modules/lucide-static/font/lucide.woff2",
            "static/icons/lucide.woff2",
        ),
        (
            true,
            "node_modules/lucide-static/font/lucide.ttf",
            "static/icons/lucide.ttf",
        ),
        (
            true,
            "node_modules/lucide-static/font/lucide.svg",
            "static/icons/lucide.svg",
        ),
    ];

    for (go, src, dest) in &copies {
        if *go {
            if let Ok(dest_meta) = std::fs::metadata(dest) {
                let src_meta = std::fs::metadata(src).unwrap_or_else(|_| {
                    panic!("Failed to get metadata of {src}");
                });

                if src_meta.modified().unwrap() <= dest_meta.modified().unwrap() {
                    continue;
                }
            }

            if let Err(err) = std::fs::copy(src, dest) {
                panic!("Failed to copy {src} to {dest}: {err:?}");
            }
        }
    }

    let npm_cmds = vec![
        vec!["install"],
        vec!["run", if is_debug { "build-dev" } else { "build" }],
    ];

    for cmd in &npm_cmds {
        let status = Command::new("npm").args(cmd).status().unwrap_or_else(|_| {
            panic!("Failed to run npm commands: {cmd:?}");
        });

        if !status.success() {
            panic!("Failed to run npm commands: {cmd:?}");
        }
    }

    println!("cargo:rerun-if-changed=scripts");
    println!("cargo:rerun-if-changed=style");
    println!("cargo:rerun-if-changed=templates");
    println!("cargo:rerun-if-changed=package.json");
    println!("cargo:rerun-if-changed=postcss.config.js");
    println!("cargo:rerun-if-changed=tailwind.config.js");

    println!("cargo:rustc-env=CARGO_PROFILE={profile}");
    println!("cargo:rustc-env=CARGO_BUILD_DATE={build_date}");
}
