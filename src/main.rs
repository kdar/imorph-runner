use std::{collections::HashMap, fs::File, io, path::Path, process::Stdio, time::Duration};

use anyhow::{Context, Result};
use reqwest;
use serde::Deserialize;
use time::{UtcOffset, macros::format_description};
use tokio::{
  fs,
  io::{AsyncBufReadExt as _, AsyncReadExt, AsyncWriteExt, BufReader},
  process::Command,
  time::sleep,
};
use tracing::info;
use tracing_subscriber::fmt::time::OffsetTime;
use zip::{read::ZipArchive, result::ZipResult};

mod config;

#[derive(Debug, Deserialize)]
struct ImorphEntry {
  name: String,
  wow_version: String,
  // imorph_version: String,
  url: String,
}

type RegionData = HashMap<String, Vec<ImorphEntry>>; // e.g., "Classic" -> Vec<ImorphEntry>
type RootData = HashMap<String, RegionData>; // e.g., "China" -> RegionData

fn unzip_file(zip_path: impl AsRef<Path>, extract_to: &str) -> ZipResult<()> {
  let file = File::open(zip_path)?;
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
      let mut outfile = File::create(&outpath)?;
      io::copy(&mut file, &mut outfile)?;
      info!(name = name, "Extracted");
    }
  }

  Ok(())
}

pub async fn run_command(
  command: impl AsRef<std::ffi::OsStr>,
  args: &[&str],
) -> io::Result<std::process::ExitStatus> {
  let mut child = Command::new(command)
    .args(args)
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()?;

  let stdout = child.stdout.take().expect("Failed to capture stdout");
  let stderr = child.stderr.take().expect("Failed to capture stderr");

  let mut stdout_reader = BufReader::new(stdout).lines();
  let mut stderr_reader = BufReader::new(stderr).lines();

  // Spawn tasks to read stdout and stderr simultaneously
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

  let status = child.wait().await?;
  stdout_task.await?;
  stderr_task.await?;

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
fn enable_ansi_support() {
  use std::io::{self};

  use windows_sys::Win32::System::Console::{
    ENABLE_VIRTUAL_TERMINAL_PROCESSING, GetConsoleMode, GetStdHandle, STD_OUTPUT_HANDLE,
    SetConsoleMode,
  };

  unsafe {
    let handle = GetStdHandle(STD_OUTPUT_HANDLE);
    let mut mode = 0;
    if GetConsoleMode(handle, &mut mode) != 0 {
      SetConsoleMode(handle, mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING);
    }
  }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  #[cfg(windows)]
  enable_ansi_support();

  init_tracing();

  let cfg_file = "config.toml";

  info!(path = cfg_file, "Loading config");
  let cfg = config::load(cfg_file)?;
  let output_dir = Path::new(&cfg.output_directory);

  info!(path = &cfg.output_directory, "Creating output directory");
  tokio::fs::create_dir_all(&cfg.output_directory)
    .await
    .context("Failed to create output directory")?;

  info!("Fetching API");
  let response = reqwest::get(&cfg.api).await.context("Failed to call API")?;
  info!("Parsing JSON");
  let data: RootData = response.json().await.context("Failed to parse JSON")?;

  let Some(flavor_entries) = data.get(&cfg.region) else {
    return Err(format!("could not find region {}", cfg.region).into());
  };

  let Some(name_entries) = flavor_entries.get(&cfg.flavor) else {
    return Err(format!("could not find flavor {}", cfg.flavor).into());
  };

  let Some(entry) = name_entries.iter().find(|v| v.name == cfg.name) else {
    return Err(format!("could not find iMorph name of {}", cfg.name).into());
  };

  let version_path = output_dir.join("latest.txt");

  info!(path = version_path.to_str(), "Opening version file");
  let installed_version = match fs::File::open(&version_path).await {
    Ok(mut file) => {
      let mut contents = String::new();
      file.read_to_string(&mut contents).await?;
      contents
    },
    Err(e) if e.kind() == io::ErrorKind::NotFound => String::new(),
    Err(e) => return Err(Box::new(e) as Box<dyn std::error::Error>),
  };

  if installed_version.trim() == &entry.wow_version {
    info!(
      version = installed_version.trim(),
      "Latest iMorph already downloaded"
    );
    info!(path = "RuniMorph.exe", "Running iMorph");
    run_command(output_dir.join("RuniMorph.exe"), &[])
      .await
      .context("Failed to run command")?;
    sleep(Duration::from_secs(2)).await;
    return Ok(());
  }

  info!(
    path = output_dir.join("download.zip").to_str(),
    "Removing old downloaded zip"
  );
  std::fs::remove_file(output_dir.join("download.zip")).ok();

  info!(
    cmd = format!(
      "{} {}",
      &cfg.mega_get,
      output_dir.join("download.zip").to_str().unwrap(),
    ),
    "Running mega-get"
  );
  let mut cmd = Command::new(&cfg.mega_get);
  cmd
    .arg(&entry.url)
    .arg(output_dir.join("download.zip"))
    .stdout(Stdio::piped())
    .stderr(Stdio::piped());
  let child = cmd.spawn().context("Failed to call mega-get")?;

  child.wait_with_output().await?;

  info!(
    path = output_dir.join("download.zip").to_str(),
    "Unzipping downloaded zip"
  );
  unzip_file(output_dir.join("download.zip"), "download").context("Failed to unzip file")?;

  info!(path = "RuniMorph.exe", "Running iMorph");
  run_command(output_dir.join("RuniMorph.exe"), &[])
    .await
    .context("Failed to run RuniMorph.exe")?;

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

  sleep(Duration::from_secs(2)).await;

  Ok(())
}
