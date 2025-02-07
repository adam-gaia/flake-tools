use anyhow::bail;
use anyhow::Result;
use clap::Parser;
use derivation::Derivation;
use log::debug;
use log::warn;
use s_string::s;
use std::env;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
mod cli;
mod derivation;
use cli::Cli;

fn back_search(dir: &Path, file: &str) -> Result<PathBuf> {
    let mut parent = Some(dir);
    while let Some(dir) = parent {
        let test = dir.join(file);
        if test.is_file() {
            return Ok(test);
        }
        parent = dir.parent();
    }
    bail!("Unable to find a flake.nix in the directory ancestory");
}

fn system() -> &'static str {
    match (env::consts::ARCH, env::consts::OS) {
        ("x86_64", "linux") => "x86_64-linux",
        ("aarch64", "linux") => "aarch64-linux",
        ("x86_64", "macos") => "x86_64-darwin",
        ("aarch64", "macos") => "aarch64-darwin",
        ("x86", "windows") => "i686-windows",
        ("x86_64", "windows") => "x86_64-windows",
        ("aarch64", "windows") => "aarch64-windows",
        _ => "unknown",
    }
}

fn nix(args: &[String]) -> Result<()> {
    let nix = which::which("nix")?;
    debug!("Running command {} {:?}", nix.display(), args);
    Command::new(&nix).args(args).spawn()?;
    Ok(())
}

#[derive(Debug)]
struct Flake {
    system: String,
    root: PathBuf,
}

impl Flake {
    pub fn discover() -> Result<Self> {
        let cwd = env::current_dir()?;
        let root = back_search(&cwd, "flake.nix")?
            .parent()
            .unwrap()
            .to_path_buf();
        if !root.join(".git").is_dir() {
            warn!("Flake is not a git repo");
        }

        let system = system().to_string();

        Ok(Self { root, system })
    }

    pub fn check(&self, derivation: Option<Derivation>) -> Result<()> {
        let mut args = vec![s!("flake"), s!("check")];
        if let Some(derivation) = derivation {
            args.push(derivation.to_string("checks", &self.system));
        }
        nix(&args)?;
        Ok(())
    }

    pub fn build(&self, derivation: Option<Derivation>) -> Result<()> {
        let mut args = vec![s!("build")];
        if let Some(derivation) = derivation {
            args.push(derivation.to_string("packages", &self.system));
        }
        nix(&args)
    }

    pub fn run(&self, derivation: Option<Derivation>) -> Result<()> {
        let mut args = vec![s!("run")];
        if let Some(derivation) = derivation {
            args.push(derivation.to_string("packages", &self.system));
        }
        nix(&args)
    }

    pub fn show(&self) -> Result<()> {
        // TODO: Take a filter and show all packages/devshells/etc and filter by system
        let args = vec![s!("flake"), s!("show")];
        nix(&args)
    }
}

fn main() -> Result<()> {
    env_logger::init();
    let args = Cli::parse();

    let flake = Flake::discover()?;

    match args.command() {
        cli::Command::Build { derivation } => {
            flake.build(derivation)?;
        }
        cli::Command::Check { derivation } => {
            flake.check(derivation)?;
        }
        cli::Command::Run { derivation } => {
            flake.run(derivation)?;
        }
        cli::Command::Show {} => {
            flake.show()?;
        }
    }

    Ok(())
}
