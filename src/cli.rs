use crate::derivation::Derivation;
use clap::{Parser, Subcommand};

#[derive(Debug, Subcommand, Clone)]
pub enum Command {
    Build { derivation: Option<Derivation> },
    Check { derivation: Option<Derivation> },
    Run { derivation: Option<Derivation> },
    Show {},
}

impl Default for Command {
    fn default() -> Self {
        Command::Show {}
    }
}

#[derive(Debug, Parser)]
pub struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

impl Cli {
    pub fn command(&self) -> Command {
        match self.command.as_ref() {
            Some(command) => command.clone(),
            None => Command::default(),
        }
    }
}
