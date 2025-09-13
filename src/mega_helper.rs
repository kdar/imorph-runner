use std::path::Path;

use anyhow::{Result, anyhow};
use regex::Regex;
use tokio::process::Command;

fn get_nth_component(path_str: &str, n: usize) -> Option<&str> {
  let path = Path::new(path_str);

  path.components()
        .filter_map(|c| c.as_os_str().to_str()) // Convert components to &str
        .nth(n)
}

pub async fn get_mega_download_links<P: AsRef<Path>>(
  mega_folder: &str,
  wow_version: &str,
) -> Result<Vec<crate::ImorphEntry>> {
  let http_client = reqwest::Client::new();
  let m = mega::Client::builder().build(http_client).unwrap();

  let nodes = m.fetch_public_nodes(mega_folder).await?;
  for root in nodes.roots() {
    println!("{}", root.name());
    for child in root.children() {
      let c = nodes.get_node_by_handle(child).unwrap();
      println!("{}", c.name());
      if c.name() == "retail" {
        for child2 in c.children() {
          let c = nodes.get_node_by_handle(child2).unwrap();
          println!("{}", c.name());

          if c.name() == "iMorph-1.5.166(menu)[11.2.0.63163].zip" {
            let file = File::create("test.zip").await?;
            m.download_node(c, &mut file.compat_write()).await?;
          }
        }
      }
    }
  }

  let output = Command::new(mega_path.as_ref().join("mega-find.bat"))
    .args(&[
      "/iMorph",
      format!("--pattern=*[*{}].zip", wow_version).as_str(),
    ])
    .output()
    .await
    .expect("Failed to execute command");

  if !output.status.success() {
    let stderr_str = String::from_utf8(output.stderr).expect("Error output was not valid UTF-8");
    return Err(anyhow!("mega-find: {}", stderr_str));
  }

  let stdout_str = String::from_utf8(output.stdout).expect("Output was not valid UTF-8");
  let mut all_downloads = vec![];

  let app_regex = Regex::new(r".*/iMorph-([\d\.]+)?(\((.*?)\))?\[(China)? ?([\d\.]+)\]")?;
  for line in stdout_str.lines() {
    println!("{}", line);
    let line = line.trim();
    let Some(caps) = app_regex.captures(&line) else {
      return Err(anyhow!("regex failed to match iMorph line: {}", line));
    };

    let (Some(imorph_version), feature, region, Some(wow_version)) = (
      caps.get(1).map(|v| v.as_str()),
      caps.get(3).map(|v| v.as_str()),
      caps.get(4).map(|v| v.as_str()),
      caps.get(5).map(|v| v.as_str()),
    ) else {
      return Err(anyhow!("unexpected iMorph line: {}", line));
    };

    let feature: crate::Feature = feature.unwrap_or("").parse()?;
    let region: crate::Region = region.unwrap_or("").parse()?;

    all_downloads.push(crate::ImorphEntry {
      feature,
      imorph_version: imorph_version.to_string(),
      wow_version: wow_version.to_string(),
      url: line.to_string(),
      region,
      product: match get_nth_component(line, 2) {
        Some("retail") => crate::Product::WoW,
        Some("cata") => crate::Product::WoWClassicEra,
        Some("classic") => crate::Product::WoWClassic,
        v => return Err(anyhow!("unknown wow flavor: {:?}", v)),
      },
    });
  }

  Ok(all_downloads)
}

// pub async fn get_mega_download_links<P: AsRef<Path>>(
//   mega_path: P,
//   wow_version: &str,
// ) -> Result<Vec<crate::ImorphEntry>> {
//   let output = Command::new(mega_path.as_ref().join("mega-find.bat"))
//     .args(&[
//       "/iMorph",
//       format!("--pattern=*[*{}].zip", wow_version).as_str(),
//     ])
//     .output()
//     .await
//     .expect("Failed to execute command");

//   if !output.status.success() {
//     let stderr_str = String::from_utf8(output.stderr).expect("Error output was not valid UTF-8");
//     return Err(anyhow!("mega-find: {}", stderr_str));
//   }

//   let stdout_str = String::from_utf8(output.stdout).expect("Output was not valid UTF-8");
//   let mut all_downloads = vec![];

//   let app_regex = Regex::new(r".*/iMorph-([\d\.]+)?(\((.*?)\))?\[(China)? ?([\d\.]+)\]")?;
//   for line in stdout_str.lines() {
//     println!("{}", line);
//     let line = line.trim();
//     let Some(caps) = app_regex.captures(&line) else {
//       return Err(anyhow!("regex failed to match iMorph line: {}", line));
//     };

//     let (Some(imorph_version), feature, region, Some(wow_version)) = (
//       caps.get(1).map(|v| v.as_str()),
//       caps.get(3).map(|v| v.as_str()),
//       caps.get(4).map(|v| v.as_str()),
//       caps.get(5).map(|v| v.as_str()),
//     ) else {
//       return Err(anyhow!("unexpected iMorph line: {}", line));
//     };

//     let feature: crate::Feature = feature.unwrap_or("").parse()?;
//     let region: crate::Region = region.unwrap_or("").parse()?;

//     all_downloads.push(crate::ImorphEntry {
//       feature,
//       imorph_version: imorph_version.to_string(),
//       wow_version: wow_version.to_string(),
//       url: line.to_string(),
//       region,
//       product: match get_nth_component(line, 2) {
//         Some("retail") => crate::Product::WoW,
//         Some("cata") => crate::Product::WoWClassicEra,
//         Some("classic") => crate::Product::WoWClassic,
//         v => return Err(anyhow!("unknown wow flavor: {:?}", v)),
//       },
//     });
//   }

//   Ok(all_downloads)
// }
