//! Command-line options

use std::path::PathBuf;

/// Parsed command-line arguments
#[derive(clap::Parser, Debug)]
#[structopt(about)]
pub struct Opts {
    /// The paths of filenames to unixize
    #[structopt(required = true)]
    pub paths: Vec<PathBuf>,

    /// Program flags
    #[structopt(flatten)]
    pub flags: Flags,
}

/// Parsed command-line flags
#[derive(clap::Parser, Debug, Copy, Clone)]
#[structopt(about)]
pub struct Flags {
    /// Recursively unixize filenames in directories. If some of the specified
    /// paths are directories, unf will operate recursively on their contents.
    #[structopt(long, short)]
    pub recursive: bool,

    /// Do not interactively prompt to rename each file.
    #[structopt(long, short)]
    pub force: bool,

    /// Do not actually rename files. Only print the renames that would happen.
    #[structopt(long, short, conflicts_with = "force")]
    pub dry_run: bool,
}
