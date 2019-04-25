#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate clap;
#[macro_use]
extern crate failure;
extern crate regex;
extern crate walkdir;

use regex::Regex;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

type Result<T> = std::result::Result<T, Box<std::error::Error>>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unixize_filename_str() {
        let f = unixize_filename_str;
        assert_eq!(f("verbatim"), "verbatim");
        assert_eq!(f("__trim____"), "trim");
        assert_eq!(f("__a___b___c__"), "a_b_c");
        assert_eq!(f("  a   b   c  "), "a_b_c");
        assert_eq!(f("a-b-c"), "a-b-c");
        assert_eq!(f("🤔😀😃😄😁😆😅emojis.txt"), "emojis.txt");
        assert_eq!(f("Game (Not Pirated 😉).rar"), "Game_Not_Pirated.rar");
    }
}

// Clean up a string representing a filename, replacing
// unix-unfriendly characters (like spaces, parentheses, etc.) See the
// unit test for examples.
fn unixize_filename_str(fname: &str) -> String {
    lazy_static! {
        static ref RE_INVAL_CHR: Regex = Regex::new("[^a-zA-Z0-9._-]").unwrap();
        static ref RE_UND_DUP: Regex = Regex::new("_+").unwrap();
        static ref RE_UND_DOT: Regex = Regex::new("_+\\.").unwrap();
    }
    // Replace all invalid characters with underscores
    let s = RE_INVAL_CHR.replace_all(fname, "_");
    // Remove duplicate underscores
    let s = RE_UND_DUP.replace_all(&s, "_");
    // Remove underscores before dot ('.')
    let s = RE_UND_DOT.replace_all(&s, ".");
    // Remove leading and trailing underscores
    let s = s.trim_matches('_');
    s.to_string()
}

// Use clap crate to parse arguments
fn parse_args() -> clap::ArgMatches<'static> {
    app_from_crate!()
        // TODO: make usage more descriptive
        .args_from_usage(
            "<PATH>... 'The paths of filenames to unixize'
             -r --recursive 'Recursively unixize filenames in directories'
             -q --quiet 'Do not write to stdout'
             -d --dryrun 'Do not rename files, only print names'
             -s --follow-symlinks 'Follow symbolic links'",
        )
        .get_matches()
}

// Unixize the filename(s) specified by a path, according to the
// supplied arguments
fn unixize_filename(path: &Path, args: &clap::ArgMatches<'static>) -> Result<()> {
    lazy_static! {
        static ref CWD: PathBuf = std::env::current_dir().unwrap();
    }
    let parent = path.parent().unwrap_or(&CWD);
    let basename = &path
        .file_name()
        .ok_or(format_err!("path '{}' has no basename", path.display()))?
        .to_string_lossy();
    let new_basename = unixize_filename_str(basename);

    if args.is_present("recursive") {
        let follow_links = args.is_present("follow_symlinks");
        let dir_iter = WalkDir::new(path).follow_links(follow_links);
        for ent in dir_iter {
            unixize_filename(ent?.path(), args)?;
        }
    }

    // Skip files that already have unix-friendly names; this is done
    // after recursive handling because unix-friendly directory names
    // might have non-unix-friendly filenames inside
    if basename != &new_basename {
        return Ok(());
    }

    let new_path = parent.join(new_basename);
    if !args.is_present("quiet") {
        println!("'{}' -> '{}'", path.display(), new_path.display());
    }
    if !args.is_present("dryrun") {
        std::fs::rename(path, new_path)?;
    }

    Ok(())
}

fn try_main() -> Result<()> {
    let args = parse_args();

    // Here unwrap() is safe because PATH is a required argument
    for path in args.values_of("PATH").unwrap().map(Path::new) {
        unixize_filename(path, &args)?;
    }

    Ok(())
}

fn main() {
    if let Err(err) = try_main() {
        eprintln!("Error: {}", err);
        std::process::exit(1);
    }
}
