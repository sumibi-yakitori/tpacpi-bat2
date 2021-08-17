use std::{
  error::Error,
  ffi::OsStr,
  io::Write,
  path::{Path, PathBuf},
  process::{Command, ExitStatus, Stdio},
  str::FromStr,
};

type Result<T = ()> = std::result::Result<T, Box<dyn Error>>;

fn main() -> Result {
  if cfg!(target_os = "linux") {
    let user_name = std::env::args().nth(1).unwrap_or(whoami::username());
    let tpacpi_repo_path = PathBuf::from_str(&format!("/home/{}/acpi_call", user_name))?;
    if std::env::args().nth(1).is_none() {
      install_tpacpi_bat()?;
      install_self()?;
      create_dependent_repo(&tpacpi_repo_path)?;
    }
    apply_kernel_mod(tpacpi_repo_path).expect("apply_kernel_mod");
    // beep()?;
    // TODO: run(&["cat", "/sys/class/power_supply/BAT{}/capacity"])
  }
  else {
    println!("If you run this program on an OS other than linux, it will not do anything.");
  }
  Ok(())
}

fn apply_kernel_mod(tpacpi_repo_path: impl AsRef<Path>) -> Result {
  let path = std::env::current_dir()?.canonicalize()?;
  std::env::set_current_dir(tpacpi_repo_path)?;

  run(&["make"])?;
  run(&["sudo", "make", "install"])?;
  run(&["sudo", "depmod"])?;
  run(&["sudo", "modprobe", "acpi_call"])?;

  std::env::set_current_dir(path)?;

  Ok(())
}

fn create_dependent_repo(local_repo_path: impl AsRef<Path>) -> Result {
  const ACPI_CALL_GIT_REPO: &'static str = "git://github.com/nix-community/acpi_call.git";

  let local_repo_path = local_repo_path.as_ref();
  if !local_repo_path.exists() {
    run(&[
      "git",
      "clone",
      ACPI_CALL_GIT_REPO,
      &local_repo_path.to_string_lossy(),
    ])?;
  }

  let path = std::env::current_dir()?.canonicalize()?;
  std::env::set_current_dir(local_repo_path)?;

  run(&["git", "fetch"])?;
  run(&["git", "reset", "HEAD", "--hard"])?;
  run(&["git", "clean", "-fd"])?;
  run(&["git", "checkout", "origin/master"])?;

  std::env::set_current_dir(path)?;

  Ok(())
}

fn install_self() -> Result {
  let name = std::env!("CARGO_CRATE_NAME");
  run(&["sudo", "cp", name, "/usr/bin"])?;

  // Add the job to the crontab of the superuser running with root privileges.
  let upstream_stdout = Command::new("sudo")
    .arg("crontab")
    .arg("-l")
    .stdout(Stdio::piped())
    .spawn()?
    .wait_with_output()?;
  let mut content = String::from_utf8_lossy(&upstream_stdout.stdout).into_owned();
  let job = format!("@reboot /usr/bin/{} --apply", name);
  if content.lines().into_iter().all(|s| *s != job) {
    // A new line is required before EOF in crontab.
    content = format!("{}\n{}\n", content, job);
  }
  let mut process = Command::new("sudo")
    .arg("crontab")
    .arg("-")
    .stdin(Stdio::piped())
    .spawn()?;

  let mut stdin = process.stdin.take().expect("unreachable");
  stdin.write_all(content.as_bytes())?;
  stdin.flush()?;
  drop(stdin); // = Ctrl + D
  process.wait()?;

  Ok(())
}

// fn beep() -> Result {
//   use beep::beep;
//   beep(440)?;
//   std::thread::sleep(std::time::Duration::from_millis(500));
//   Ok(())
// }

fn install_tpacpi_bat() -> Result {
  run(&["sudo", "cp", "tpacpi-bat", "/usr/bin"])?;

  Ok(())
}

fn run(args: &[impl AsRef<OsStr>]) -> Result<ExitStatus> {
  let (cmd, args) = args.split_at(1);
  match cmd.get(0) {
    Some(cmd) => Ok(Command::new(cmd).args(args).spawn()?.wait()?),
    None => Err("Invalid command".into()),
  }
}
