use anyhow::{anyhow, Result};

use std::ffi::OsStr;
use std::process::{Child, Command, ExitStatus};

pub fn run_command_spawn<I, S>(bin: &str, args: I) -> Result<Child>
where
    I: IntoIterator<Item = S> + std::fmt::Debug,
    S: AsRef<OsStr>,
{
    let child = Command::new(bin)
        .env("RUST_BACKTRACE", "full")
        .args(args)
        .spawn()
        .expect("run command");
    Ok(child)
}

pub fn run_command_status<I, S>(bin: &str, args: I) -> Result<ExitStatus>
where
    I: IntoIterator<Item = S> + std::fmt::Debug,
    S: AsRef<OsStr>,
{
    let status = Command::new(bin)
        .env("RUST_BACKTRACE", "full")
        .args(args)
        .status()
        .expect("run command");
    Ok(status)
}

pub fn run_command_output<I, S>(bin: &str, args: I) -> Result<(String, String)>
where
    I: IntoIterator<Item = S> + std::fmt::Debug,
    S: AsRef<OsStr>,
{
    let output = Command::new(bin)
        .env("RUST_BACKTRACE", "full")
        .args(args)
        .output()
        .expect("run command");

    if !output.status.success() {
        Err(anyhow!(
            "{}",
            String::from_utf8_lossy(output.stderr.as_slice())
        ))
    } else {
        let stdout = String::from_utf8_lossy(output.stdout.as_slice()).to_string();
        let stderr = String::from_utf8_lossy(output.stderr.as_slice()).to_string();
        Ok((stdout, stderr))
    }
}
