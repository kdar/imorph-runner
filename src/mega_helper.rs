use std::path::Path;

use anyhow::Result;
use anyhow::anyhow;
use megalib::Node;
use megalib::PublicFolder;
use regex::Regex;

pub struct MegaHelper {
  folder: PublicFolder,
}

impl MegaHelper {
  pub async fn try_new(url: &str) -> Result<Self> {
    let folder = megalib::open_folder(url).await?;

    Ok(Self {
      folder,
    })
  }

  pub async fn fetch_entries(
    &self,
    region: crate::Region,
    product: crate::Product,
    feature: crate::Feature,
    wow_version: &str,
  ) -> Result<Vec<crate::ImorphEntry>> {
    let product_path = match product {
      crate::Product::WoW => "retail",
      crate::Product::WoWClassic => "classic",
      crate::Product::WoWClassicEra => "cata",
      crate::Product::WoWBeta => "beta",
    };

    // Get root folder
    let root = self
      .folder
      .nodes()
      .first()
      .ok_or_else(|| anyhow!("Unable to find root in public folder."))?;
    let root_path = root.path().unwrap_or("/");
    let path = format!("{}/{}", root_path, product_path);

    let app_regex = Regex::new(r"iMorph-([\d\.]+)(\((.*?)\))?\[(China)? ?([\d\.]+)\].zip")?;
    let mut all_downloads = vec![];

    // List all files in the product directory
    for node in self.folder.list(&path, false) {
      let line = node.name.clone();
      let Some(caps) = app_regex.captures(&line) else {
        return Err(anyhow!("regex failed to match iMorph line: {}", line));
      };

      let (Some(imorph_version), entry_feature, entry_region, Some(entry_wow_version)) = (
        caps.get(1).map(|v| v.as_str()),
        caps.get(3).map(|v| v.as_str()),
        caps.get(4).map(|v| v.as_str()),
        caps.get(5).map(|v| v.as_str()),
      ) else {
        return Err(anyhow!("unexpected iMorph line: {}", line));
      };

      let entry_feature: crate::Feature = entry_feature.unwrap_or("").parse()?;
      let entry_region: crate::Region = entry_region.unwrap_or("").parse()?;

      if feature != entry_feature || entry_region != region || entry_wow_version != wow_version {
        continue;
      }

      all_downloads.push(crate::ImorphEntry {
        // feature: entry_feature,
        imorph_version: imorph_version.to_string(),
        wow_version: entry_wow_version.to_string(),
        node: node.clone(),
        // region: entry_region,
        // product,
      });
    }

    Ok(all_downloads)
  }

  pub async fn download<P: AsRef<Path>>(&self, node: &Node, output_path: P) -> Result<()> {
    let mut file = std::fs::File::create(output_path)?;
    self.folder.download(node, &mut file).await?;
    Ok(())
  }
}
