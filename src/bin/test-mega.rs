use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
  let folder =
    megalib::open_folder("https://mega.nz/folder/XQdwFJTR#X8VNWdap7eKtIvmPbpW6sA").await?;
  let root_path = folder.nodes().first().unwrap().path().unwrap_or("/");

  for node in folder.list(&format!("{}/retail", root_path), false) {
    if node.name == "iMorph-1.5.278(net)[12.1.0.69814].zip" {
      println!("Node handle: {}", node.handle);
      break;
    }
  }

  Ok(())
}
