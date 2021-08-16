use std::{
  error::Error,
  ffi::OsStr,
  path::{Path, PathBuf},
  process::ExitStatus,
  str::FromStr,
};

type Result<T = ()> = std::result::Result<T, Box<dyn Error>>;

fn main() -> Result {
  if cfg!(target_os = "linux") {
    let tpacpi_repo_path = PathBuf::from_str(&format!("/home/{}/acpi_call", whoami::username()))?;
    if std::env::args().nth(1).is_none() {
      install_tpacpi_bat()?;
      install_self()?;
      create_dependent_repo(tpacpi_repo_path)?;
    }
    apply_kernel_mod(tpacpi_repo_path)?;
  }
  else {
    println!("If you run this program on an OS other than linux, it will not do anything.");
  }
  Ok(())
}

fn apply_kernel_mod(tpacpi_repo_path: impl AsRef<Path>) -> Result {
  std::env::set_current_dir(tpacpi_repo_path)?;
  run(&["make"])?;
  run(&["sudo", "make", "install"])?;
  run(&["sudo", "depmod"])?;
  run(&["sudo", "modprobe", "acpi_call"])?;
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

  std::env::set_current_dir(local_repo_path)?;

  run(&["git", "fetch"])?;
  run(&["git", "reset", "HEAD", "--hard"])?;
  run(&["git", "clean", "-fd"])?;
  run(&["git", "checkout", "origin/master"])?;

  Ok(())
}

fn install_self() -> Result {
  let name = std::env!("CARGO_CRATE_NAME");
  run(&["sudo", "cp", name, "/usr/bin"])?;
  run(&[
    "echo",
    &format!("@reboot /usr/bin/{} --apply", name),
    "|",
    "sudo",
    "crontab",
  ])?;

  Ok(())
}

fn install_tpacpi_bat() -> Result {
  run(&["sudo", "cp", "tpacpi-bat", "/usr/bin"])?;

  Ok(())
}

fn run(args: &[impl AsRef<OsStr>]) -> Result<ExitStatus> {
  use std::process::Command;
  let (cmd, args) = args.split_at(1);
  match cmd.get(0) {
    Some(cmd) => Ok(Command::new(cmd).args(args).spawn()?.wait()?),
    None => Err("Invalid command".into()),
  }
}
