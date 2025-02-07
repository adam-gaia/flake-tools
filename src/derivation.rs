use std::fmt::Display;
use std::str::FromStr;
use thiserror::Error;
use winnow::combinator::alt;
use winnow::prelude::*;
use winnow::stream::AsChar;
use winnow::token::take_while;
use winnow::Result;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("unable to parse derivation")]
    UnableToParse,
}

fn word(s: &mut &str) -> Result<String> {
    take_while(1.., |c: char| c.is_alphanum() || c == '_' || c == '-')
        .map(|s: &str| s.to_string())
        .parse_next(s)
}

fn partial(s: &mut &str) -> Result<PartialDerivation> {
    word.map(|name| PartialDerivation { name }).parse_next(s)
}

fn remote(s: &mut &str) -> Result<RemoteDerivation> {
    let protocol = word.parse_next(s)?;
    let _ = ":".parse_next(s)?;
    let path = word.parse_next(s)?;
    Ok(RemoteDerivation { protocol, path })
}

fn local(s: &mut &str) -> Result<LocalDerivation> {
    // TODO: this could be done better by peeking
    let _ = ".#".parse_next(s)?;
    let path = word.parse_next(s)?;
    Ok(LocalDerivation { path })
}

fn derivation(s: &mut &str) -> Result<Derivation> {
    alt((
        local.map(|l| Derivation::Local(l)),
        remote.map(|r| Derivation::Remote(r)),
        partial.map(|p| Derivation::Partial(p)),
    ))
    .parse_next(s)
}

#[derive(Debug, Clone)]
pub struct RemoteDerivation {
    protocol: String,
    path: String,
}

impl Display for RemoteDerivation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.protocol, self.path)
    }
}

#[derive(Debug, Clone)]
pub struct LocalDerivation {
    path: String,
}

impl Display for LocalDerivation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, ".#{}", self.path)
    }
}

#[derive(Debug, Clone)]
pub struct PartialDerivation {
    name: String,
}

impl PartialDerivation {
    fn to_string(&self, ttype: &str, system: &str) -> String {
        format!(".#{}.{}.{}", ttype, system, self.name)
    }
}

#[derive(Debug, Clone)]
pub enum Derivation {
    Remote(RemoteDerivation),
    Local(LocalDerivation),
    Partial(PartialDerivation),
}

impl Derivation {
    pub fn to_string(&self, system: &str, ttype: &str) -> String {
        match self {
            Derivation::Remote(r) => r.to_string(),
            Derivation::Local(l) => l.to_string(),
            Derivation::Partial(p) => p.to_string(system, ttype),
        }
    }
}

impl FromStr for Derivation {
    type Err = ParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        derivation.parse(s).map_err(|_| ParseError::UnableToParse)
    }
}
