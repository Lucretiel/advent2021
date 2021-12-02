include!(concat!(env!("OUT_DIR"), "/generated.rs"));

mod library;

use std::{
    fs::File,
    io::{self, Read},
    num::ParseIntError,
    path::PathBuf,
    str::FromStr,
};

use anyhow::Context;
use structopt::StructOpt;
use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum DayError {
    #[error("Failed to parse day")]
    Parse(#[from] ParseIntError),

    #[error("{0} is not an Advent Puzzle Day")]
    BadDay(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Part {
    Part1,
    Part2,
}

#[derive(Debug, Clone, Error)]
pub enum PartError {
    #[error("Failed to parse part")]
    Parse(#[from] ParseIntError),

    #[error("{0} is not an Advent Puzzle Part; must be 1 or 2")]
    BadPart(u8),
}

impl FromStr for Part {
    type Err = PartError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let value: u8 = s.parse()?;

        match value {
            1 => Ok(Part::Part1),
            2 => Ok(Part::Part2),
            value => Err(PartError::BadPart(value)),
        }
    }
}

#[derive(StructOpt)]
struct Args {
    /// The advent of code day to solve
    #[structopt(short, long)]
    day: Day,

    /// Which part of the day to solve
    #[structopt(short, long)]
    part: Part,

    /// If given, read input from this file
    #[structopt(short, long, conflicts_with = "string")]
    file: Option<PathBuf>,

    /// If given, use this as the puzzle input directly
    #[structopt(short, long, conflicts_with = "file")]
    string: Option<String>,
}

fn main() -> anyhow::Result<()> {
    let args: Args = Args::from_args();

    let buf = match args.string {
        Some(buf) => buf,
        None => {
            let mut buf = String::new();
            match args.file {
                Some(file) => File::open(&file)
                    .with_context(|| format!("failed to open file: {:?}", file.display()))?
                    .read_to_string(&mut buf)
                    .context("failed to read puzzle input from file")?,
                None => io::stdin()
                    .read_to_string(&mut buf)
                    .context("failed to read puzzle input from stdin")?,
            };
            buf
        }
    };

    run_solution(args.day, args.part, &buf)
}
