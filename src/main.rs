#[macro_use]
extern crate lazy_static;

mod filename_parts;
mod mem_fs;
mod opts;

use filename_parts::FilenameParts;
use opts::Flags;
use opts::Opts;

use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::ffi::OsString;
use std::path::Path;
use std::path::PathBuf;

use deunicode::deunicode;
use promptly::prompt_default;
use regex::Regex;
use rsfs::DirEntry;
use rsfs::GenFS;
use rsfs::Metadata;
use structopt::StructOpt;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[cfg(test)]
mod tests {
    use super::*;

    use tempfile::TempDir;

    #[test]
    fn test_unixize_filename_str() {
        let f = unixize_filename_str;
        assert_eq!(f("verbatim"), "verbatim");
        assert_eq!(f("__trim____"), "trim");
        assert_eq!(f("__a___b___c__"), "a_b_c");
        assert_eq!(f("  a   b   c  "), "a_b_c");
        assert_eq!(f("a-b-c"), "a-b-c");
        assert_eq!(
            f("🤔😀😃😄😁😆😅emojis.txt"),
            "thinking_grinning_smiley_smile_grin_laughing_sweat_smile_emojis.txt"
        );
        assert_eq!(f("Æneid"), "AEneid");
        assert_eq!(f("étude"), "etude");
        assert_eq!(f("北亰"), "Bei_Jing");
        assert_eq!(f("げんまい茶"), "genmaiCha");
        assert_eq!(f("🦄☣"), "unicorn_biohazard");
        assert_eq!(f("Game (Not Pirated 😉).rar"), "Game_Not_Pirated_wink.rar");
        assert_eq!(f("--fake-flag"), "fake-flag");
        assert_eq!(f("Évidemment"), "Evidemment");
        assert_eq!(f("àà_y_ü"), "aa_y_u");
    }

    #[test]
    fn test_resolve_collision() {
        let fs = rsfs::disk::FS;
        let root = TempDir::new().unwrap();
        let root = root.path();
        test_resolve_collision_fs(&fs, root);

        let fs = rsfs::mem::FS::new();
        let root = Path::new("/");
        test_resolve_collision_fs(&fs, root);
    }

    fn test_resolve_collision_fs<FS: GenFS>(fs: &FS, root: &Path) {
        // Helper function taking a collider filename returning a
        // string representing the resolved collision
        let f = |filename: &str| -> String {
            let path = root.join(filename);
            fs.create_file(&path).unwrap();

            resolve_collision(fs, root, &path)
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .to_string()
        };

        assert_eq!(f("a"), "a_000");
        assert_eq!(f("b_000"), "b_001");
        assert_eq!(f("c.txt"), "c_000.txt");
        assert_eq!(f("d_333.txt"), "d_334.txt");
        assert_eq!(f("e_999.txt"), "e_1000.txt");
        assert_eq!(f("e_1000.txt"), "e_1000_000.txt");
        assert_eq!(f("z___222.txt"), "z___223.txt");
        assert_eq!(f(".x._._._222.txt"), ".x._._._223.txt");
    }
}

/// Clean up a string representing a filename, replacing
/// unix-unfriendly characters (like spaces, parentheses, etc.) See the
/// unit tests for examples.
fn unixize_filename_str(fname: &str) -> String {
    lazy_static! {
        static ref RE_INVAL_CHR: Regex = Regex::new("[^a-zA-Z0-9._-]").unwrap();
        static ref RE_UND_DUP: Regex = Regex::new("_+").unwrap();
        static ref RE_UND_DOT: Regex = Regex::new("_+\\.").unwrap();
    }

    // Replace all UNICODE characters with their ASCII counterparts
    let s = deunicode(fname);
    // Replace all remaining invalid characters with underscores
    let s = RE_INVAL_CHR.replace_all(&s, "_");
    // Remove duplicate underscores
    let s = RE_UND_DUP.replace_all(&s, "_");
    // Remove underscores before dot ('.')
    let s = RE_UND_DOT.replace_all(&s, ".");
    // Remove leading and trailing underscores and hyphens
    let s = s.trim_matches(|c| c == '_' || c == '-');
    s.to_string()
}

fn read_children_names<FS: GenFS>(fs: &FS, cwd: &Path, dir: &Path) -> Result<BTreeSet<OsString>> {
    let children_names = fs
        .read_dir(cwd.join(dir))?
        .map(|result_ent| result_ent.map(|ent| ent.file_name()))
        .collect::<std::io::Result<BTreeSet<OsString>>>()?;
    Ok(children_names)
}

/// Like `unixize_path()`, but only operate on children of `dir`
fn unixize_children<FS: GenFS>(fs: &FS, cwd: &Path, dir: &Path, flags: Flags) -> Result<()> {
    for file_name in read_children_names(fs, cwd, dir)? {
        let path = dir.join(file_name);
        unixize_path(fs, cwd, &path, flags)?;
    }
    Ok(())
}

/// Unixize the filename(s) specified by a path, according to the
/// supplied arguments
fn unixize_path<FS: GenFS>(fs: &FS, cwd: &Path, path: &Path, flags: Flags) -> Result<()> {
    let parent = path.parent().unwrap_or(cwd);
    let basename = &path.file_name().map(OsStr::to_string_lossy);
    let basename = match basename {
        Some(s) => s,
        // If the path has no basename (for example, if it's `.` or `..`), only
        // unixize children
        None => return unixize_children(fs, cwd, path, flags),
    };
    let new_basename = unixize_filename_str(basename);

    let stat = fs.metadata(cwd.join(path))?;
    let is_dir = stat.is_dir();
    let should_prompt = !flags.force && !flags.dry_run;

    // Determine whether to recurse, possibly by prompting the user
    let recurse = flags.recursive
        && is_dir
        && (!should_prompt || {
            let msg = format!("descend into directory '{}'?", path.display());
            prompt_default(msg, false)?
        });

    if recurse {
        unixize_children(fs, cwd, path, flags)?;
    }

    // Skip files that already have unix-friendly names; this is done
    // after recursive handling because unix-friendly directory names
    // might have non-unix-friendly filenames inside
    if basename == &new_basename {
        return Ok(());
    }

    let new_path = parent.join(new_basename);
    let new_path = resolve_collision(fs, cwd, &new_path);
    let rename_prefix = if flags.dry_run {
        "would rename"
    } else {
        "rename"
    };
    let msg = format!(
        "{} '{}' -> '{}'",
        rename_prefix,
        path.display(),
        new_path.display()
    );
    if should_prompt {
        // Interactively prompt whether to rename the file, skipping
        // if the user says no
        let msg = format!("{}?", msg);
        if !prompt_default(msg, false)? {
            return Ok(());
        }
    } else {
        // Log rename non-interactively
        println!("{}", msg);
    }

    fs.rename(cwd.join(path), cwd.join(new_path))?;
    Ok(())
}

/// Split, modify, and re-merge filename to increment the
/// collision-resolving number, or create it if non-existent
fn inc_filename_num(filename: &str) -> String {
    let FilenameParts { stem, num, ext } = FilenameParts::from_filename(filename);
    let num = match num {
        Some(val) => Some(val + 1),
        None => Some(0),
    };
    FilenameParts { stem, num, ext }.merge()
}

/// Check if the target path can be written to without clobbering an
/// existing file. If it can't, change it to a unique name. Note that
/// this function requires that the filename is non-empty and valid
/// UTF-8.
fn resolve_collision<FS: GenFS>(fs: &FS, cwd: &Path, path: &Path) -> PathBuf {
    if path_exists(fs, cwd, path) {
        let filename = path
            .file_name()
            .expect("filename is empty")
            .to_str()
            .expect("filename is not valid UTF-8");
        let filename = inc_filename_num(filename);
        let path = path.with_file_name(filename);

        // Recursively resolve the new filename. This is how the
        // collision-resolving number is incremented.
        resolve_collision(fs, cwd, &path)
    } else {
        // File does not exist; we're done!
        path.to_path_buf()
    }
}

/// Returns `true` if the path points at an existing entity.
fn path_exists<FS, P1, P2>(fs: &FS, cwd: P1, path: P2) -> bool
where
    FS: GenFS,
    P1: AsRef<Path>,
    P2: AsRef<Path>,
{
    let cwd = cwd.as_ref();
    let path = path.as_ref();
    fs.metadata(cwd.join(path)).is_ok()
}

fn unixize_paths<FS: GenFS>(fs: &FS, cwd: &Path, paths: &[PathBuf], flags: Flags) -> Result<()> {
    for path in paths {
        unixize_path(fs, cwd, &path, flags)?;
    }
    Ok(())
}

/// Run `unf` with parsed command-line arguments in `opts`, returning any error
fn main_opts(opts: Opts) -> Result<()> {
    let cwd = std::env::current_dir()?;

    if opts.flags.dry_run {
        // If using `--dry-run`, load the file tree into an in-memory filesystem
        // and use that instead of the real filesystem. This is required for the
        // collision handling to work.
        let fs = mem_fs::load(&opts.paths)?;
        unixize_paths(&fs, &cwd, &opts.paths, opts.flags)
    } else {
        let fs = rsfs::disk::FS;
        unixize_paths(&fs, &cwd, &opts.paths, opts.flags)
    }
}

/// Run `unf` with passed program arguments, returning any error
fn try_main() -> Result<()> {
    main_opts(Opts::from_args())
}

/// Run `unf` with passed program arguments, printing any error
fn main() {
    if let Err(err) = try_main() {
        eprintln!("unf: error: {}", err);
        std::process::exit(1);
    }
}
