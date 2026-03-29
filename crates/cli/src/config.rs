use std::{
    io::Write,
    path::{Path, PathBuf},
};

use anyhow::Context;
use config::{builder::DefaultState, ConfigBuilder, Environment, File, FileFormat};
use serde::{Deserialize, Serialize};

static EXAMPLE_CONFIG: &str = include_str!("../../../etc/example/config.toml");

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    /// Path to the storage of configured hosts
    pub hosts_path: PathBuf,
}

impl Config {
    pub fn load<P: AsRef<Path>>(config_path: Option<P>) -> anyhow::Result<Self> {
        let config_dir = get_config_dir();
        let data_dir = get_data_dir();

        fs_err::create_dir_all(&config_dir)
            .with_context(|| format!("Failed to create config directory: {config_dir:?}"))?;

        fs_err::create_dir_all(&data_dir)
            .with_context(|| format!("Failed to create data directory: {data_dir:?}"))?;

        let config_path = if let Some(config_path) = config_path {
            config_path.as_ref().to_path_buf()
        } else {
            config_dir.join("config.toml")
        };

        let builder = Self::builder(config_path, &data_dir)?;
        let config = builder.build()?;
        let config = config
            .try_deserialize::<Self>()
            .context("failed to deserialize config")?;

        let config = Config {
            hosts_path: expand(&config.hosts_path)?,
        };

        Ok(config)
    }

    fn builder(
        config_path: PathBuf,
        data_dir: &Path,
    ) -> anyhow::Result<ConfigBuilder<DefaultState>> {
        let hosts_path = data_dir.join("hosts.json");

        let mut builder = ConfigBuilder::<DefaultState>::default()
            .set_default("hosts_path", hosts_path.to_str())?
            .add_source(Environment::with_prefix("PARCEL_").separator("_"));

        if config_path.exists() {
            builder = builder.add_source(File::new(
                config_path.to_str().expect("valid config path"),
                FileFormat::Toml,
            ));
        } else {
            let mut file = fs_err::File::create(&config_path)
                .with_context(|| format!("Failed to create config file: {config_path:?}"))?;
            file.write_all(EXAMPLE_CONFIG.as_bytes())
                .with_context(|| format!("Failed to write example config to: {config_path:?}"))?;
        }

        Ok(builder)
    }
}

fn get_home_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        let home = std::env::var("USERPROFILE").expect("%USERPROFILE% not found");
        PathBuf::from(home)
    }

    #[cfg(not(target_os = "windows"))]
    {
        let home = std::env::var("HOME").expect("$HOME not found");
        PathBuf::from(home)
    }
}

fn get_config_dir() -> PathBuf {
    std::env::var("XDG_CONFIG_HOME")
        .map_or_else(|_| get_home_dir().join(".config"), PathBuf::from)
        .join("parcel")
}

fn get_data_dir() -> PathBuf {
    std::env::var("XDG_DATA_HOME")
        .map_or_else(
            |_| get_home_dir().join(".local").join("share"),
            PathBuf::from,
        )
        .join("parcel")
}

fn expand(path: &PathBuf) -> anyhow::Result<PathBuf> {
    let result = shellexpand::full(
        path.to_str()
            .with_context(|| format!("Failed to process path: {path:?}"))?,
    )
    .with_context(|| format!("Failed to expand path: {path:?}"))?;
    Ok(PathBuf::from(result.into_owned()))
}
