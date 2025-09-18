use std::{ffi::OsStr, io::Read, path::Path, process::Command, sync::mpsc, thread, time::Duration};

use conpty::Process;
use regex::Regex;
use tracing::info;

pub fn run_command<P: AsRef<Path>>(
  cwd: P,
  command: impl AsRef<OsStr>,
  args: &[&str],
) -> anyhow::Result<u32> {
  let mut cmd = Command::new(command.as_ref());
  cmd.args(args);
  cmd.current_dir(cwd);

  let mut proc = Process::spawn(cmd)?;
  let mut reader = proc.output()?;

  let ansi_re = Regex::new(r"\x1b\[[0-9;]*[a-zA-Z]")?;

  let (tx, rx) = mpsc::channel();

  // Spawn thread to read from PTY output
  thread::spawn(move || {
    let mut buf = [0u8; 1024];
    loop {
      match reader.read(&mut buf) {
        Ok(0) => {
          // End of stream
          let _ = tx.send(None);
          break;
        },
        Ok(n) => {
          let _ = tx.send(Some(buf[..n].to_vec()));
        },
        Err(_) => {
          let _ = tx.send(None);
          break;
        },
      };
    }
  });

  let mut line_buffer = Vec::new();

  loop {
    match rx.recv_timeout(Duration::from_millis(100)) {
      Ok(Some(data)) => {
        for byte in data {
          line_buffer.push(byte);
          if byte == b'\n' {
            let line = String::from_utf8_lossy(&line_buffer);
            let clean = ansi_re.replace_all(&line, "");
            info!("[imorph] {}", clean.trim_end_matches(&['\r', '\n'][..]));
            line_buffer.clear();
          }
        }
      },
      Ok(None) => break, // Reader thread signaled EOF
      Err(mpsc::RecvTimeoutError::Timeout) => {
        if !proc.is_alive() {
          // No more data coming, process is dead
          break;
        }
      },
      Err(_) => break, // Channel disconnected
    }
  }

  // Flush remaining buffer
  if !line_buffer.is_empty() {
    let line = String::from_utf8_lossy(&line_buffer);
    let clean = ansi_re.replace_all(&line, "");
    info!("[imorph] {}", clean.trim_end());
  }

  let exit_code = proc.wait(None)?;
  Ok(exit_code)
}
