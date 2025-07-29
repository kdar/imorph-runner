use std::{
  fs::{read, write},
  path::Path,
};

/// Pattern to match: mov ecx, 5000 (0x1388), call dword ptr [Sleep]
const PATTERN: &[u8] = &[0xB9, 0x88, 0x13, 0x00, 0x00, 0xFF, 0x15]; // 7 bytes

pub fn patch_specific_sleep_call<P: AsRef<Path>>(path: P) -> anyhow::Result<()> {
  let mut buf = read(&path)?;

  // Collect all offsets that need to be patched
  let patch_offsets: Vec<usize> = {
    let mut offsets = Vec::new();
    for i in 0..=buf.len().saturating_sub(11) {
      if &buf[i..i + 7] == PATTERN {
        // println!("found at offset {}", i);
        offsets.push(i);
      }
    }
    offsets
  };

  // Now apply all patches
  for offset in patch_offsets {
    for j in 0..11 {
      buf[offset + j] = 0x90;
    }
  }

  write(path, &buf)?;
  Ok(())
}
