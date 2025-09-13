use std::collections::HashMap;

use anyhow::{Context, Result, anyhow};
use regex::Regex;
use reqwest::Client;
use scraper::{Html, Selector};

#[derive(Debug, Deserialize, Serialize)]
pub struct ImorphEntry {
  feature: String,
  wow_version: String,
  imorph_version: String,
  url: String,
}

pub type RegionData = HashMap<String, Vec<ImorphEntry>>; // e.g., "Classic" -> Vec<ImorphEntry>
pub type RootData = HashMap<String, RegionData>; // e.g., "China" -> RegionData

// Method doesn't work because of cloudflare bot protection. There are workarounds but not worth it.
// "https://www.ownedcore.com/forums/wow-classic/wow-classic-bots-programs/935744-imorph-wow-classic.html"
pub async fn get_ownedcore_download_links(url: &str) -> Result<crate::RootData> {
  let client = Client::new();
  let html_content = client.get(url).send().await?.text().await?;
  let document = Html::parse_document(&html_content);

  let message_body_selector = Selector::parse(".postcontent")
    .map_err(|e| anyhow!("Could not parse .postcontent selector: {e}"))?;
  let post_body = document
    .select(&message_body_selector)
    .next()
    .context("Failed to find the first post's message body")?;

  let link_text_regex = Regex::new(r"iMorph - ([\d\.]+) ?(\(.*\))? \[([\d\.]+)\]")?;

  let mut all_downloads: crate::RootData = HashMap::new();

  let mut current_region = "global".to_string();
  let versions = ["classic era", "classic", "retail"];
  let mut version_index = 0;
  let mut downloads = vec![];

  for element in post_body.child_elements() {
    let elem = element.value();
    let elem_text = element.text().collect::<String>().trim().to_string();

    match elem.name() {
      "b" => {
        if elem_text.contains("Archive:") {
          break;
        }
      },
      "a" => {
        if let Some(caps) = link_text_regex.captures(&elem_text) {
          if let (Some(imorph_ver), Some(wow_ver_full), Some(href)) =
            (caps.get(1), caps.get(3), elem.attr("href"))
          {
            let mut name = "iMorph".to_string();
            if let Some(link_type) = caps.get(2) {
              let link_type_str = link_type
                .as_str()
                .trim_matches(|c| c == '(' || c == ')')
                .to_string();
              name = format!("iMorph {}", link_type_str);
            }

            let download = crate::ImorphEntry {
              feature: name,
              imorph_version: imorph_ver.as_str().to_string(),
              wow_version: wow_ver_full.as_str().to_string(),
              url: href.to_string(),
            };

            downloads.push(download);

            if downloads.len() == 3 {
              let entry = all_downloads
                .entry(current_region.clone())
                .or_insert_with(HashMap::new);
              entry.insert(versions[version_index].to_owned(), downloads);
              downloads = vec![];
              version_index += 1;
              if entry.len() == 3 {
                current_region = "china".to_string();
                version_index = 0;
              }
            }
          }
        }
      },
      _ => {},
    }
  }

  Ok(all_downloads)
}
