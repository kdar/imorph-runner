pub mod proto {
  pub mod defs {
    include!(concat!(env!("OUT_DIR"), "/productdb.defs.rs"));
  }
}

use std::io::Cursor;

use prost::Message;
pub use proto::defs;

pub fn deserialize(buf: &[u8]) -> Result<defs::ProductDb, prost::DecodeError> {
  defs::ProductDb::decode(&mut Cursor::new(buf))
}
