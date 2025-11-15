pub async fn run_command(
  command: impl AsRef<std::ffi::OsStr>,
  args: &[&str],
) -> Result<std::process::ExitStatus> {
  let mut child = Command::new(command)
    .args(args)
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    // .stdout(Stdio::inherit())
    // .stderr(Stdio::inherit())
    .spawn()?;

  let stdout = child.stdout.take().expect("Failed to capture stdout");
  let stderr = child.stderr.take().expect("Failed to capture stderr");

  let mut stdout_reader = BufReader::new(stdout).lines();
  let mut stderr_reader = BufReader::new(stderr).lines();

  // let stdout_task = tokio::spawn(async move {
  //   let mut buf = [0; 1024];
  //   loop {
  //     match stdout.read(&mut buf).await {
  //       Ok(0) => break, // EOF
  //       Ok(n) => {
  //         let chunk = String::from_utf8_lossy(&buf[..n]);
  //         print!("[stdout] {}", chunk);
  //       },
  //       Err(e) => {
  //         eprintln!("Error reading stdout: {}", e);
  //         break;
  //       },
  //     }
  //   }
  // });

  let stdout_task = tokio::spawn(async move {
    while let Ok(Some(line)) = stdout_reader.next_line().await {
      info!("[imorph] {}", line);
    }
  });

  let stderr_task = tokio::spawn(async move {
    while let Ok(Some(line)) = stderr_reader.next_line().await {
      info!("[imorph] {}", line);
    }
  });

  stdout_task.await?;
  stderr_task.await?;
  let status = child.wait().await?;

  Ok(status)
}
