use std::collections::HashMap;

use tokio::{
  fs::File,
  io::{self, AsyncBufReadExt, BufReader},
};

async fn parse_build_info(file_path: &str) -> io::Result<Option<String>> {
  let file = File::open(file_path).await?;
  let reader = BufReader::new(file);
  let mut lines = reader.lines();

  // Read header line
  let header_line = match lines.next_line().await? {
    Some(line) => line,
    None => return Ok(None),
  };

  let headers: Vec<&str> = header_line.split('|').collect();

  // Process each line
  while let Some(line) = lines.next_line().await? {
    let fields: Vec<&str> = line.split('|').collect();

    if fields.len() != headers.len() {
      continue;
    }

    let entry: HashMap<_, _> = headers.iter().zip(fields.iter()).collect();

    if let Some(&"wow") = entry.get(&"ProductCode") {
      if let Some(&version) = entry.get(&"Version") {
        return Ok(Some(version.to_string()));
      }
    }
  }

  Ok(None)
}

async fn build_info() {
  let build_info_path = r"C:\Program Files (x86)\World of Warcraft\.build.info";

  match parse_build_info(build_info_path).await {
    Ok(Some(version)) => println!("Detected WoW Version: {}", version),
    Ok(None) => println!("WoW version not found in .build.info"),
    Err(e) => eprintln!("Error reading file: {}", e),
  }
}
