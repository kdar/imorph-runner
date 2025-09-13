use std::{fs, io};

use anyhow::{Context, Result, anyhow};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct Config {
  pub region: crate::Region,
  pub product: crate::Product,
  pub feature: crate::Feature,
  pub output_directory: String,
  pub mega_folder: String,
}

impl Default for Config {
  fn default() -> Self {
    Self {
      region: crate::Region::Global,
      product: crate::Product::WoW,
      feature: crate::Feature::Net,
      output_directory: "download".to_string(),
      mega_folder: "https://mega.nz/folder/XQdwFJTR#X8VNWdap7eKtIvmPbpW6sA".to_string(),
    }
  }
}

pub fn load(path: &str) -> Result<Config> {
  let toml_str = match fs::read_to_string(path) {
    Ok(content) => content,
    Err(error) => {
      if error.kind() == io::ErrorKind::NotFound {
        return Ok(Config::default());
      } else {
        return Err(anyhow!(error));
      }
    },
  };

  let config: Config = toml::from_str(&toml_str).context("Could not parse config")?;
  Ok(config)
}
