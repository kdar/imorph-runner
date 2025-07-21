use std::{fs, io};

use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct Config {
  pub region: String,
  pub flavor: String,
  pub name: String,
  pub output_directory: String,
  pub mega_get: String,
  pub api: String,
}

impl Default for Config {
  fn default() -> Self {
    Self {
      region: "Global".to_string(),
      flavor: "Retail".to_string(),
      name: "iMorph Net".to_string(),
      output_directory: "download".to_string(),
      mega_get: "mega-get.bat".to_string(),
      api: "https://www.imorph.dev/api/apps".to_string(),
    }
  }
}

pub fn load(path: &str) -> Result<Config, Box<dyn std::error::Error>> {
  let toml_str = match fs::read_to_string(path) {
    Ok(content) => content,
    Err(error) => {
      if error.kind() == io::ErrorKind::NotFound {
        return Ok(Config::default());
      } else {
        return Err(Box::new(error));
      }
    },
  };

  let config: Config = toml::from_str(&toml_str).context("Could not parse config")?;
  Ok(config)
}
