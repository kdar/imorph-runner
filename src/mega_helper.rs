use std::path::Path;

use anyhow::{Result, anyhow};
use regex::Regex;
use tokio::fs::File;
use tokio_util::compat::TokioAsyncWriteCompatExt;

pub struct MegaHelper {
  nodes: mega::Nodes,
  client: mega::Client,
}

impl MegaHelper {
  pub async fn try_new(url: &str) -> Result<Self> {
    let http_client = reqwest::Client::new();
    let m = mega::Client::builder().build(http_client)?;
    let nodes = m.fetch_public_nodes(url).await?;

    Ok(Self {
      nodes,
      client: m,
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
    };

    let node = self
      .nodes
      .get_node_by_path(&format!("iMorph/{}", product_path))
      .ok_or_else(|| anyhow!("could not find iMorph/{} path", product_path))?;

    let app_regex = Regex::new(r"iMorph-([\d\.]+)(\((.*?)\))?\[(China)? ?([\d\.]+)\].zip")?;
    let mut all_downloads = vec![];
    for child_handle in node.children() {
      let n = self
        .nodes
        .get_node_by_handle(child_handle)
        .ok_or_else(|| anyhow!("unable to get node by handle"))?;

      let line = n.name();
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
        feature: entry_feature,
        imorph_version: imorph_version.to_string(),
        wow_version: entry_wow_version.to_string(),
        handle: n.handle().to_string(),
        region: entry_region,
        product,
      });
    }

    Ok(all_downloads)
  }

  pub async fn download<P: AsRef<Path>>(&self, handle: &str, output_path: P) -> Result<()> {
    let node = self
      .nodes
      .get_node_by_handle(handle)
      .ok_or_else(|| anyhow!("unable to find node with handle\"{}\"", handle))?;
    let file = File::create(output_path).await?;
    self
      .client
      .download_node(node, &mut file.compat_write())
      .await?;
    Ok(())
  }
}
