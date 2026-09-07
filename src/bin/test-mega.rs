use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let folder = megalib::open_folder("https://mega.nz/folder/XQdwFJTR#X8VNWdap7eKtIvmPbpW6sA").await?;
    let root_path = folder.nodes().first().unwrap().path().unwrap_or("/");
    
    let mut count = 0;
    for node in folder.list(&root_path, true) {
        if node.name.contains("1.5.275") {
            println!("FOUND: {} in path: {}", node.name, node.path().unwrap_or_default());
            count += 1;
        }
    }
    println!("Found {} nodes matching 1.5.275", count);

    Ok(())
}
