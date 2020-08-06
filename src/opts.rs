//! Command-line options

use std::path::PathBuf;

use structopt::StructOpt;

/// Parsed command-line arguments
#[derive(StructOpt, Debug)]
#[structopt(about)]
pub struct Opts {
    /// The paths of filenames to unixize
    pub paths: Vec<PathBuf>,

    /// Program flags
    #[structopt(flatten)]
    pub flags: Flags,
}

/// Parsed command-line flags
#[derive(StructOpt, Debug, Copy, Clone)]
#[structopt(about)]
pub struct Flags {
    /// Recursively unixize filenames in directories. If some of the specified
    /// paths are directories, unf will operate recursively on their contents.
    #[structopt(long, short)]
    pub recursive: bool,

    /// Do not interactively prompt to rename each file.
    #[structopt(long, short)]
    pub force: bool,
}
