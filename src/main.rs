use std::{fmt, fs::File as StdFile, io, path::Path, str::FromStr};

use anyhow::{Context, Result, anyhow};
use semver::Version;
use serde::{Deserialize, Serialize};
use time::{UtcOffset, macros::format_description};
use tokio::{
  fs,
  io::{AsyncReadExt, AsyncWriteExt},
};
use tracing::{error, info};
use tracing_subscriber::fmt::time::OffsetTime;
use zip::{read::ZipArchive, result::ZipResult};

mod buildinfo;
mod config;
mod mega_helper;
mod productdb;
mod pty;

#[derive(PartialEq, Eq, Copy, Clone, Debug, Deserialize, Serialize)]
enum Region {
  #[serde(rename = "global")]
  Global,
  #[serde(rename = "china")]
  China,
}

impl FromStr for Region {
  type Err = anyhow::Error;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match s.to_lowercase().as_str() {
      "" => Ok(Region::Global),
      "china" => Ok(Region::China),
      _ => Err(anyhow!("could not parse region \"{}\"", s)),
    }
  }
}

#[derive(PartialEq, Eq, Copy, Clone, Debug, Deserialize, Serialize)]
enum Product {
  #[serde(rename = "wow")]
  WoW,
  #[serde(rename = "wow_classic")]
  WoWClassic,
  #[serde(rename = "wow_classic_era")]
  WoWClassicEra,
  #[serde(rename = "wow_beta")]
  WoWBeta,
}

impl fmt::Display for Product {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Product::WoW => write!(f, "wow"),
      Product::WoWClassic => write!(f, "wow_classic"),
      Product::WoWClassicEra => write!(f, "wow_classic_era"),
      Product::WoWBeta => write!(f, "wow_beta"),
    }
  }
}

#[derive(PartialEq, Eq, Copy, Clone, Debug, Deserialize, Serialize)]
enum Feature {
  #[serde(rename = "")]
  None,
  #[serde(rename = "net")]
  Net,
  #[serde(rename = "menu")]
  Menu,
}

impl FromStr for Feature {
  type Err = anyhow::Error;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match s.to_lowercase().as_str() {
      "" => Ok(Feature::None),
      "net" => Ok(Feature::Net),
      "menu" => Ok(Feature::Menu),
      _ => Err(anyhow!("could not parse feature \"{}\"", s)),
    }
  }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ImorphEntry {
  feature: Feature,
  wow_version: String,
  imorph_version: String,
  region: Region,
  product: Product,
  handle: String,
}

// pub type RegionData = HashMap<String, Vec<ImorphEntry>>; // e.g., "Classic" -> Vec<ImorphEntry>
// pub type RootData = HashMap<String, RegionData>; // e.g., "China" -> RegionData

// static PRODUCT_MAP: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
//   let mut map = HashMap::new();
//   map.insert("retail", "wow");
//   map.insert("classic", "wow_classic");
//   map.insert("classic era", "wow_classic_era");
//   map
// });

/// Extracts a zip file to the specified directory
fn unzip_file(zip_path: impl AsRef<Path>, extract_to: &str) -> ZipResult<()> {
  let file = StdFile::open(zip_path)?;
  let mut archive = ZipArchive::new(file)?;

  for i in 0..archive.len() {
    let mut file = archive.by_index(i)?;
    let name = file.name().to_owned();
    let outpath = Path::new(extract_to).join(file.mangled_name());

    // Check if imorph.conf already exists, and skip if so
    if name == "imorph.conf" && outpath.exists() {
      info!(name = name, "Skipping existing file");
      continue;
    }

    if name.ends_with('/') {
      std::fs::create_dir_all(&outpath)?;
    } else {
      if let Some(parent) = outpath.parent() {
        std::fs::create_dir_all(parent)?;
      }
      let mut outfile = StdFile::create(&outpath)?;
      io::copy(&mut file, &mut outfile)?;
      info!(name = name, "Extracted");
    }
  }

  Ok(())
}

fn init_tracing() {
  let timer_format = format_description!("[year]-[month]-[day] [hour]:[minute]");
  let local_offset = UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC);

  let timer = OffsetTime::new(local_offset, timer_format);

  tracing_subscriber::fmt()
    // .with_max_level(tracing::Level::DEBUG)
    .with_writer(std::io::stdout) // forces flush after every write
    .with_target(false)
    .with_timer(timer)
    .with_span_events(tracing_subscriber::fmt::format::FmtSpan::NONE)
    .compact()
    .init();
}

#[cfg(windows)]
pub fn enable_ansi_support() -> Result<()> {
  use windows::Win32::{
    Foundation::HANDLE,
    System::Console::{
      CONSOLE_MODE, ENABLE_VIRTUAL_TERMINAL_PROCESSING, GetConsoleMode, GetStdHandle,
      STD_OUTPUT_HANDLE, SetConsoleMode,
    },
  };

  unsafe {
    let handle: HANDLE = GetStdHandle(STD_OUTPUT_HANDLE)?;

    let mut mode = CONSOLE_MODE::default();
    GetConsoleMode(handle, &mut mode)?;
    SetConsoleMode(handle, mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING)?;

    Ok(())
  }
}

/// Sets up the environment (ANSI support, tracing)
fn setup_environment() -> Result<()> {
  #[cfg(windows)]
  enable_ansi_support()?;
  init_tracing();
  Ok(())
}

/// Ensures the output directory exists
async fn ensure_output_directory(path: &str) -> Result<()> {
  info!(path = path, "Creating output directory");
  tokio::fs::create_dir_all(path)
    .await
    .context("Failed to create output directory")?;
  Ok(())
}

/// Retrieves the WoW build info for the specified product
async fn get_wow_build_info(product: Product) -> Result<buildinfo::BuildInfoEntry> {
  info!("Finding WoW install path");
  let install_path = buildinfo::find_wow_install_path(product)?;
  let buildinfo_path = install_path.join(".build.info");
  
  info!(
    path = format!("{}", buildinfo_path.as_os_str().to_str().unwrap()),
    "Reading WoW build info"
  );
  
  let buildinfos = buildinfo::get_build_infos(&buildinfo_path).await?;
  
  if buildinfos.is_empty() {
    return Err(anyhow!(
      "No build info found in {:?}. Do you have WoW installed?",
      buildinfo_path
    ));
  }

  buildinfos
    .into_iter()
    .find(|v| v.product == product)
    .ok_or_else(|| anyhow!("Could not find product: {}", product))
}

/// Finds the latest iMorph entry matching the criteria
async fn find_latest_imorph_entry(
  mh: &mega_helper::MegaHelper,
  region: Region,
  product: Product,
  feature: Feature,
  wow_version: &str,
) -> Result<ImorphEntry> {
  info!("Fetching latest iMorph info");
  let mut entries = mh
    .fetch_entries(region, product, feature, wow_version)
    .await?;

  if entries.is_empty() {
    return Err(anyhow!(
      "iMorph has not been released for the latest WoW version={}.",
      wow_version
    ));
  }

  // Find the entry with the greatest imorph_version according to semantic versioning
  let parse_version = |v: &str| Version::parse(v).unwrap_or_else(|_| Version::new(0, 0, 0));
  
  let max_index = entries
    .iter()
    .enumerate()
    .max_by(|(_, a), (_, b)| {
      parse_version(&a.imorph_version).cmp(&parse_version(&b.imorph_version))
    })
    .map(|(idx, _)| idx)
    .unwrap_or(0);

  Ok(entries.remove(max_index))
}

/// Reads the version file and returns (imorph_version, wow_version)
async fn read_version_file(version_path: &Path) -> Result<(String, String)> {
  info!(path = version_path.to_str(), "Opening version file");
  
  match fs::File::open(version_path).await {
    Ok(mut file) => {
      let mut contents = String::new();
      file.read_to_string(&mut contents).await?;
      let contents = contents.trim().to_string();
      Ok(
        contents
          .split_once("|")
          .map(|v| (v.0.to_string(), v.1.to_string()))
          .unwrap_or((String::new(), String::new()))
      )
    },
    Err(e) if e.kind() == io::ErrorKind::NotFound => Ok((String::new(), String::new())),
    Err(e) => Err(anyhow!(e)),
  }
}

/// Checks if we already have the latest version downloaded
fn is_already_downloaded(
  downloaded_imorph_version: &str,
  downloaded_wow_version: &str,
  entry: &ImorphEntry,
  buildinfo: &buildinfo::BuildInfoEntry,
) -> bool {
  downloaded_imorph_version == entry.imorph_version
    && downloaded_wow_version == buildinfo.version
}

/// Downloads and extracts the iMorph zip file
async fn download_and_extract_imorph(
  mh: &mega_helper::MegaHelper,
  entry: &ImorphEntry,
  output_dir: &Path,
) -> Result<()> {
  let download_path = output_dir.join("download.zip");

  info!(path = download_path.to_str(), "Removing old downloaded zip");
  std::fs::remove_file(&download_path).ok();

  info!(
    imorph_version = entry.imorph_version,
    wow_version = entry.wow_version,
    "Downloading iMorph"
  );
  mh.download(&entry.handle, &download_path).await?;

  info!(path = download_path.to_str(), "Unzipping downloaded zip");
  unzip_file(&download_path, "download").context("Failed to unzip file")?;

  Ok(())
}

/// Updates the version file with the current versions
async fn update_version_file(version_path: &Path, entry: &ImorphEntry) -> Result<()> {
  info!(
    path = version_path.to_str(),
    version = entry.wow_version,
    "Updating version file"
  );
  
  let mut file = fs::File::create(version_path)
    .await
    .context("Failed to create version file")?;
  
  file
    .write_all(format!("{}|{}", entry.imorph_version, entry.wow_version).as_bytes())
    .await
    .context("Failed to write version data")?;
  
  Ok(())
}

/// Runs the iMorph executable
fn run_imorph(output_dir: &Path, cmd_path: &Path) -> Result<()> {
  info!(path = cmd_path.to_str(), "Running iMorph");
  pty::run_command(output_dir, cmd_path, &[], "[imorph] ")
    .context("Failed to run command")?;
  Ok(())
}

async fn run(cfg: &config::Config) -> Result<()> {
  setup_environment()?;
  ensure_output_directory(&cfg.output_directory).await?;

  let output_dir = Path::new(&cfg.output_directory);
  let buildinfo = get_wow_build_info(cfg.product).await?;

  let mh = mega_helper::MegaHelper::try_new(&cfg.mega_folder).await?;
  let entry = find_latest_imorph_entry(
    &mh,
    cfg.region,
    cfg.product,
    cfg.feature,
    &buildinfo.version,
  )
  .await?;

  let version_path = output_dir.join("latest.txt");
  let (downloaded_imorph_version, downloaded_wow_version) =
    read_version_file(&version_path).await?;

  let cmd_path = output_dir.join("RuniMorph.exe");
  
  if is_already_downloaded(
    &downloaded_imorph_version,
    &downloaded_wow_version,
    &entry,
    &buildinfo,
  ) {
    info!(
      imorph_version = downloaded_imorph_version,
      wow_version = downloaded_wow_version,
      "Already have the latest iMorph that targets this WoW version"
    );
    run_imorph(output_dir, &cmd_path)?;
    return Ok(());
  }

  download_and_extract_imorph(&mh, &entry, output_dir).await?;
  update_version_file(&version_path, &entry).await?;
  run_imorph(output_dir, &cmd_path)?;

  Ok(())
}

/// Runs all commands configured for a given trigger
fn run_commands_for_trigger(cfg: &config::Config, trigger: &str) {
  let commands = cfg.commands_for_trigger(trigger);

  if commands.is_empty() {
    return;
  }

  for cmd in commands {
    let args: Vec<&str> = cmd.args.iter().map(|s| s.as_str()).collect();
    let args_str = format!("{:?}", args);

    info!(
      trigger = trigger,
      command = cmd.path,
      args = %args_str,
      "Running command",
    );

    if let Err(e) = pty::run_command(".", &cmd.path, &args, "[cmd] ") {
      error!(
        command = cmd.path,
        trigger = trigger,
        error = %e,
        "Failed to run command"
      );
    }
  }
}

#[tokio::main]
async fn main() {
  // Load config first so we have it available for error handling
  let cfg_file = "config.toml";
  let cfg = config::load_or_default(cfg_file);

  match run(&cfg).await {
    Ok(_) => {
      run_commands_for_trigger(&cfg, "after_success");
    },
    Err(e) => {
      error!("{}", e);
      run_commands_for_trigger(&cfg, "after_error");
    },
  };
}
