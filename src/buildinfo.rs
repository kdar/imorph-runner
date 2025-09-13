use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use csv::ReaderBuilder;
use serde::Deserialize;
use tokio::{fs::File, io::AsyncReadExt};
use windows_registry::LOCAL_MACHINE;

#[derive(Debug, Deserialize)]
pub struct BuildInfoEntry {
  #[serde(rename = "Version!STRING:0")]
  pub version: String,
  #[serde(rename = "Product!STRING:0")]
  pub product: crate::Product,
}

pub fn find_wow_install_path() -> Result<PathBuf> {
  let key = LOCAL_MACHINE
    .open("SOFTWARE\\WOW6432Node\\Blizzard Entertainment\\World of Warcraft")
    .context("Failed to open WoW registry key. WoW may not be installed.")?;

  let value = key
    .get_string("InstallPath")
    .context("Failed to get 'InstallPath' value. The registry entry may be incomplete.")?;

  let p = PathBuf::from(value);
  if !p.exists() {
    return Err(anyhow::anyhow!(
      "WoW installation path does not exist: {:?}",
      p
    ));
  }

  Ok(p.parent().unwrap().to_owned())
}

pub async fn get_build_infos<P: AsRef<Path>>(path: P) -> Result<Vec<BuildInfoEntry>> {
  // Open the file asynchronously.
  let mut file = File::open(&path).await?;

  // Read the entire file content into a vector of bytes.
  let mut contents = Vec::new();
  file.read_to_end(&mut contents).await?;

  // Create a new CSV reader builder, reading from the in-memory buffer.
  let mut rdr = ReaderBuilder::new()
    .delimiter(b'|')
    .has_headers(true)
    .from_reader(contents.as_slice());

  // Iterate over the deserialized records and collect them into a Vec.
  let records = rdr
    .deserialize()
    .collect::<Result<Vec<BuildInfoEntry>, _>>()?;

  Ok(records)
}
