use std::{fs, io};

use anyhow::{Context, Result, anyhow};
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct CommandConfig {
  pub trigger: String,
  pub path: String,
  #[serde(default)]
  pub args: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct Config {
  pub region: crate::Region,
  pub product: crate::Product,
  pub feature: crate::Feature,
  pub output_directory: String,
  pub mega_folder: String,
  #[serde(default)]
  pub cmd: Vec<CommandConfig>,
}

impl Default for Config {
  fn default() -> Self {
    Self {
      region: crate::Region::Global,
      product: crate::Product::WoW,
      feature: crate::Feature::Net,
      output_directory: "download".to_string(),
      mega_folder: "https://mega.nz/folder/XQdwFJTR#X8VNWdap7eKtIvmPbpW6sA".to_string(),
      cmd: vec![CommandConfig {
        trigger: "after_error".to_string(),
        path: "powershell".to_string(),
        args: vec![
          "-command".to_string(),
          "Start-Sleep".to_string(),
          "-Seconds".to_string(),
          "5".to_string(),
        ],
      }],
    }
  }
}

impl Config {
  /// Get all commands that match the given trigger
  pub fn commands_for_trigger(&self, trigger: &str) -> Vec<&CommandConfig> {
    self.cmd.iter().filter(|c| c.trigger == trigger).collect()
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

/// Load config with a fallback to default on any error.
/// This ensures we always have a config available for error handling.
pub fn load_or_default(path: &str) -> Config {
  load(path).unwrap_or_else(|e| {
    eprintln!(
      "Warning: Failed to load config from {}: {}. Using defaults.",
      path, e
    );
    Config::default()
  })
}
