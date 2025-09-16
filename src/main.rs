use std::{fmt, fs::File as StdFile, io, path::Path, process::Stdio, str::FromStr};

use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};
use time::{UtcOffset, macros::format_description};
use tokio::{
  fs::{self},
  io::{AsyncBufReadExt as _, AsyncReadExt, AsyncWriteExt, BufReader},
  process::Command,
};
use tracing::info;
use tracing_subscriber::fmt::time::OffsetTime;
use zip::{read::ZipArchive, result::ZipResult};

mod buildinfo;
mod config;
mod mega_helper;
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
}

impl fmt::Display for Product {
  // The fmt method is required by the Display trait.
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    // Use a match statement to handle each enum variant.
    match self {
      Product::WoW => write!(f, "wow"),
      Product::WoWClassic => write!(f, "wow_classic"),
      Product::WoWClassicEra => write!(f, "wow_classic_era"),
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

fn unzip_file(zip_path: impl AsRef<Path>, extract_to: &str) -> ZipResult<()> {
  let file = StdFile::open(zip_path)?;
  let mut archive = ZipArchive::new(file)?;

  for i in 0..archive.len() {
    let mut file = archive.by_index(i)?;
    let name = file.name().to_owned();

    let outpath = Path::new(extract_to).join(file.mangled_name());

    // Check if imorph.conf already exists, and skip if so.
    if name == "imorph.conf" && outpath.exists() {
      println!("⏭️ Skipping existing file: {}", name);
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

pub async fn run_command(
  command: impl AsRef<std::ffi::OsStr>,
  args: &[&str],
) -> Result<std::process::ExitStatus> {
  let mut child = Command::new(command)
    .args(args)
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    // .stdout(Stdio::inherit())
    // .stderr(Stdio::inherit())
    .spawn()?;

  let stdout = child.stdout.take().expect("Failed to capture stdout");
  let stderr = child.stderr.take().expect("Failed to capture stderr");

  let mut stdout_reader = BufReader::new(stdout).lines();
  let mut stderr_reader = BufReader::new(stderr).lines();

  // let stdout_task = tokio::spawn(async move {
  //   let mut buf = [0; 1024];
  //   loop {
  //     match stdout.read(&mut buf).await {
  //       Ok(0) => break, // EOF
  //       Ok(n) => {
  //         let chunk = String::from_utf8_lossy(&buf[..n]);
  //         print!("[stdout] {}", chunk);
  //       },
  //       Err(e) => {
  //         eprintln!("Error reading stdout: {}", e);
  //         break;
  //       },
  //     }
  //   }
  // });

  let stdout_task = tokio::spawn(async move {
    while let Ok(Some(line)) = stdout_reader.next_line().await {
      info!("[imorph] {}", line);
    }
  });

  let stderr_task = tokio::spawn(async move {
    while let Ok(Some(line)) = stderr_reader.next_line().await {
      info!("[imorph] {}", line);
    }
  });

  stdout_task.await?;
  stderr_task.await?;
  let status = child.wait().await?;

  Ok(status)
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

#[tokio::main]
async fn main() -> Result<()> {
  #[cfg(windows)]
  enable_ansi_support()?;

  init_tracing();

  let cfg_file = "config.toml";
  info!(path = cfg_file, "Loading config");
  let cfg = config::load(cfg_file)?;
  let output_dir = Path::new(&cfg.output_directory);

  info!(path = &cfg.output_directory, "Creating output directory");
  tokio::fs::create_dir_all(&cfg.output_directory)
    .await
    .context("Failed to create output directory")?;

  info!("Finding WoW install path");
  let p = buildinfo::find_wow_install_path()?;
  let buildinfo_path = p.join(".build.info");
  info!(
    path = format!("{}", buildinfo_path.as_os_str().to_str().unwrap()),
    "Reading WoW build info"
  );
  let buildinfos = buildinfo::get_build_infos(&buildinfo_path).await?;
  if buildinfos.len() == 0 {
    return Err(anyhow!(
      "No build info found in {:?}. Do you have WoW installed?",
      buildinfo_path
    ));
  }

  let Some(buildinfo) = buildinfos
    .iter()
    .filter(|&v| v.product == cfg.product)
    .next()
  else {
    return Err(anyhow!("Could not find product: {}", cfg.product));
  };

  let version_path = output_dir.join("latest.txt");
  info!(path = version_path.to_str(), "Opening version file");
  let downloaded_imorph_wow_version = match fs::File::open(&version_path).await {
    Ok(mut file) => {
      let mut contents = String::new();
      file.read_to_string(&mut contents).await?;
      contents.trim().to_string()
    },
    Err(e) if e.kind() == io::ErrorKind::NotFound => String::new(),
    Err(e) => return Err(anyhow!(e)),
  };

  let cmd_path = output_dir.join("RuniMorph.exe");
  if downloaded_imorph_wow_version == buildinfo.version {
    // patch::patch_specific_sleep_call("download/RuniMorph.exe")?;
    info!(
      version = downloaded_imorph_wow_version.trim(),
      "Already have the iMorph that targets this WoW version"
    );
    info!(path = cmd_path.to_str(), "Running iMorph");
    pty::run_command(cmd_path, &[]).context("Failed to run command")?;
    return Ok(());
  }

  let download_path = output_dir.join("download.zip");

  info!(path = download_path.to_str(), "Removing old downloaded zip");
  std::fs::remove_file(&download_path).ok();

  info!("Fetching latest iMorph info");
  let mh = mega_helper::MegaHelper::try_new(&cfg.mega_folder).await?;
  let mut entries = mh
    .fetch_entries(cfg.region, cfg.product, cfg.feature, &buildinfo.version)
    .await?;

  if entries.is_empty() {
    info!(
      wow_version = buildinfo.version,
      "iMorph has not been released for the latest WoW version."
    );
    return Ok(());
  }

  let entry = entries.remove(0);

  info!(
    imorph_version = entry.imorph_version,
    wow_version = entry.wow_version,
    "Downloading iMorph"
  );
  mh.download(&entry.handle, &download_path).await?;

  info!(path = download_path.to_str(), "Unzipping downloaded zip");
  unzip_file(download_path, "download").context("Failed to unzip file")?;

  info!(
    path = version_path.to_str(),
    version = entry.wow_version,
    "Updating version file"
  );
  let mut file = fs::File::create(version_path)
    .await
    .context("Failed to create version file")?;
  file
    .write_all(entry.wow_version.as_bytes())
    .await
    .context("Failed to write version data")?;

  // patch::patch_specific_sleep_call("download/RuniMorph.exe")?;
  info!(
    path = output_dir.join("RuniMorph.exe").to_str(),
    "Running iMorph"
  );
  pty::run_command(cmd_path, &[]).context("Failed to run command")?;

  Ok(())
}
