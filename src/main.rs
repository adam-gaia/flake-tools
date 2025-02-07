use clap::Parser;
use derivation::Derivation;
use eyre::bail;
use eyre::Result;
use log::debug;
use log::warn;
use s_string::s;
use std::collections::HashMap;
use std::env;
use std::path::Path;
use std::path::PathBuf;
use tokio_stream::StreamExt;
use xcommand::StdioType;
use xcommand::XCommand;
use xcommand::XStatus;
mod cli;
mod derivation;
use cli::Cli;
use serde_json::Value;

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

async fn nix(
    args: &[String],
    print_stdout: bool,
    print_stderr: bool,
) -> Result<(Vec<String>, Vec<String>, i32)> {
    let bin = which::which("nix")?;

    debug!("Running command {} {:?}", bin.display(), args);
    let mut fixed_args = Vec::with_capacity(args.len());
    for arg in args {
        fixed_args.push(arg.as_str());
    }
    let Ok(mut child) = XCommand::builder(&bin)?
        .args(&fixed_args)? // TODO: make xcommand args accept anything that derefs to a &str
        .build()
        .spawn()
    else {
        bail!("Unable to run {}", bin.display());
    };

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();

    let mut streamer = child.streamer();
    let mut stream = streamer.stream();
    while let Some(item) = stream.next().await {
        let (message_type, message) = item?;
        match message_type {
            StdioType::Stdout => {
                if print_stdout {
                    println!("[stdout]{}", &message);
                }
                stdout.push(message);
            }
            StdioType::Stderr => {
                if print_stderr {
                    println!("[stderr]{}", &message);
                }
                stderr.push(message);
            }
        }
    }

    // Grab the exit code of the process
    let Ok(XStatus::Exited(code)) = child.status().await else {
        bail!("Child process was expected to have finished");
    };

    Ok((stdout, stderr, code))
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

    pub async fn check(&self, derivation: Option<Derivation>) -> Result<()> {
        let mut args = vec![s!("flake"), s!("check")];
        if let Some(derivation) = derivation {
            args.push(derivation.to_string("checks", &self.system));
        }
        nix(&args, true, true).await?;
        Ok(())
    }

    pub async fn build(&self, derivation: Option<Derivation>) -> Result<()> {
        let mut args = vec![s!("build")];
        if let Some(derivation) = derivation {
            args.push(derivation.to_string("packages", &self.system));
        }
        nix(&args, true, true).await?;
        Ok(())
    }

    pub async fn run(&self, derivation: Option<Derivation>) -> Result<()> {
        let mut args = vec![s!("run")];
        if let Some(derivation) = derivation {
            args.push(derivation.to_string("packages", &self.system));
        }
        nix(&args, true, true).await?;
        Ok(())
    }

    pub async fn show(&self) -> Result<()> {
        // TODO: Take a filter and show all packages/devshells/etc and filter by system
        let args = vec![s!("flake"), s!("show"), s!("--json")];
        let (stdout, _, _) = nix(&args, false, false).await?;

        let foo: Value = serde_json::from_str(stdout.join("\n").trim())?;

        let mut bar = HashMap::new();
        match foo {
            Value::Object(map) => {
                for (ttype, v) in map {
                    bar.insert(ttype.clone(), Vec::new());

                    // TODO: add an option to filter types?
                    match v {
                        Value::Object(map) => {
                            for (system, v) in map {
                                if system == self.system {
                                    match v {
                                        Value::Object(map) => {
                                            for (k, v) in map {
                                                match v {
                                                    Value::Object(map) => {
                                                        let Some(name) = map.get("name") else {
                                                            continue;
                                                        };
                                                        let name = name.as_str().unwrap();
                                                        let entry = format!(
                                                            "{}.{system}.{k}: {name}",
                                                            ttype.clone()
                                                        );

                                                        let vec = bar.get_mut(&ttype).unwrap();
                                                        vec.push(entry);
                                                    }
                                                    _ => continue,
                                                }
                                            }
                                        }
                                        _ => continue,
                                    }
                                }
                            }
                        }
                        _ => continue,
                    }
                }
            }
            _ => {
                bail!("Unexpected output from nix flake show")
            }
        }

        for (k, v) in bar {
            println!("> {}", k);
            for entry in v {
                println!("  - {}", entry);
            }
        }

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    let args = Cli::parse();

    let flake = Flake::discover()?;

    match args.command() {
        cli::Command::Build { derivation } => {
            flake.build(derivation).await?;
        }
        cli::Command::Check { derivation } => {
            flake.check(derivation).await?;
        }
        cli::Command::Run { derivation } => {
            flake.run(derivation).await?;
        }
        cli::Command::Show {} => {
            flake.show().await?;
        }
    }

    Ok(())
}
