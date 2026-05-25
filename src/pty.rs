use std::ffi::OsStr;
use std::io::Read;
use std::path::Path;
use std::process::Command;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use conpty::Process;
use regex::Regex;
use tracing::info;

fn strip_osc(s: &str) -> String {
  // OSC sequences: ESC ] number ; text BEL (or ESC \)
  let osc_re = Regex::new(r"\x1b\][0-9]+;[^\x07\x1b]*(\x07|\x1b\\)").unwrap();
  osc_re.replace_all(s, "").to_string()
}

fn strip_ansi(s: &str) -> String {
  Regex::new(r"\x1b\[([\x30-\x3f]*[\x20-\x2f]*[\x40-\x7e])")
    .unwrap()
    .replace_all(s, "")
    .to_string()
}

pub fn run_command<P: AsRef<Path>>(
  cwd: P,
  command: impl AsRef<OsStr>,
  args: &[&str],
  log_prefix: &str,
) -> anyhow::Result<u32> {
  let mut cmd = Command::new(command.as_ref());
  cmd.args(args);
  cmd.current_dir(cwd);

  let mut proc = Process::spawn(cmd)?;
  let mut reader = proc.output()?;

  let (tx, rx) = mpsc::channel();

  // Spawn thread to read from PTY output
  // We need to clone the span context to use it in the thread
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
            let clean = strip_osc(&strip_ansi(&line));
            info!(
              "{}{}",
              log_prefix,
              clean.trim_end_matches(&['\r', '\n'][..])
            );
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
    let clean = strip_ansi(&line);
    info!("{}{}", log_prefix, clean.trim_end());
  }

  let exit_code = proc.wait(None)?;
  Ok(exit_code)
}
