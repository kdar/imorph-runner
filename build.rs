// build.rs
use std::io::Result;

fn main() -> Result<()> {
  if std::env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows" {
    let mut res = winresource::WindowsResource::new();
    res.set_icon("logo.ico");
    res.compile().unwrap();
  }

  prost_build::compile_protos(&["src/productdb.proto"], &["src/"])?;
  Ok(())
}
