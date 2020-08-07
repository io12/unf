#[macro_use]
extern crate lazy_static;
#[macro_use]
#[cfg(test)]
extern crate maplit;

mod filename_parts;
mod opts;

use filename_parts::FilenameParts;
use opts::Flags;
use opts::Opts;

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
    use std::collections::BTreeSet;

    use rsfs::FileType;
    use tempfile::TempDir;

    use super::*;

    /// Representation of a virtual file tree used for test cases
    type FileTree = BTreeSet<FileTreeNode>;

    #[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
    enum FileTreeNode {
        File(String),
        Dir(String, FileTree),
    }

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

    /// Scan the file structure in a path to `FileTree`
    fn scan_tree<FS: GenFS>(fs: &FS, path: &Path) -> FileTree {
        let mut tree = FileTree::new();
        for ent in fs.read_dir(path).unwrap() {
            let ent = ent.unwrap();
            let is_dir = ent.file_type().unwrap().is_dir();
            let filename = ent.file_name().into_string().unwrap();
            let ent = if is_dir {
                FileTreeNode::Dir(filename, scan_tree(fs, &ent.path()))
            } else {
                FileTreeNode::File(filename)
            };
            tree.insert(ent);
        }
        tree
    }

    /// Actually create the file structure represented by a `FileTree`
    fn create_tree<FS: GenFS>(fs: &FS, tree: FileTree, path: &Path) {
        for ent in tree {
            match ent {
                FileTreeNode::File(name) => {
                    fs.create_file(path.join(name)).unwrap();
                }
                FileTreeNode::Dir(name, ents) => {
                    let path = path.join(name);
                    fs.create_dir(&path).unwrap();
                    create_tree(fs, ents, &path);
                }
            }
        }
    }

    /// Create the file structure represented by `FileTree` in a
    /// temporary directory and return its path
    fn create_tree_tmp(tree: FileTree) -> PathBuf {
        let fs = rsfs::disk::FS;
        let path = TempDir::new().unwrap().into_path();
        create_tree(&fs, tree, &path);
        path
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

    fn filenames_to_file_tree(filenames: &[&str]) -> FileTree {
        filenames
            .iter()
            .map(|s| FileTreeNode::File(s.to_string()))
            .collect()
    }

    #[test]
    fn test_main_opts() {
        // Helper function to create a specified file structure and
        // run `unf` with the specified args. It then asserts that the
        // resulting file structure matches the expected result.
        let f = |args: &[&str], tree: FileTree, expected: FileTree| {
            let path = create_tree_tmp(tree);
            std::env::set_current_dir(&path).unwrap();

            let opts = Opts::from_iter(args);
            main_opts(opts).unwrap();

            let fs = rsfs::disk::FS;
            let result = scan_tree(&fs, &path);
            assert_eq!(expected, result);
        };

        let s = "🤔😀😃😄😁😆😅emojis.txt";
        f(
            &["unf", "-f", s],
            btreeset![FileTreeNode::File(s.to_string())],
            btreeset![FileTreeNode::File(
                "thinking_grinning_smiley_smile_grin_laughing_sweat_smile_emojis.txt".to_string()
            )],
        );

        let s = "Game (Not Pirated 😉).rar";
        f(
            &["unf", "-f", s],
            btreeset![FileTreeNode::File(s.to_string())],
            btreeset![FileTreeNode::File("Game_Not_Pirated_wink.rar".to_string())],
        );

        f(
            &["unf", "-rf", "My Files/", "My Folder"],
            btreeset![
                FileTreeNode::Dir("My Folder".to_string(), btreeset![]),
                FileTreeNode::Dir(
                    "My Files".to_string(),
                    btreeset![
                        FileTreeNode::File("Passwords :) .txt".to_string()),
                        FileTreeNode::File("Another Cool Photo.JPG".to_string()),
                        FileTreeNode::File("Wow Cool Photo.JPG".to_string()),
                        FileTreeNode::File("Cool Photo.JPG".to_string()),
                    ],
                ),
            ],
            btreeset![
                FileTreeNode::Dir("My_Folder".to_string(), btreeset![]),
                FileTreeNode::Dir(
                    "My_Files".to_string(),
                    btreeset![
                        FileTreeNode::File("Passwords.txt".to_string()),
                        FileTreeNode::File("Another_Cool_Photo.JPG".to_string()),
                        FileTreeNode::File("Wow_Cool_Photo.JPG".to_string()),
                        FileTreeNode::File("Cool_Photo.JPG".to_string()),
                    ],
                ),
            ],
        );

        let filenames = [
            "--fake-flag.txt",
            "fake-flag.txt",
            "------fake-flag.txt",
            " fake-flag.txt",
            "\tfake-flag.txt",
        ];
        f(
            &[&["unf", "-f", "--"], &filenames[..]].concat(),
            filenames_to_file_tree(&filenames),
            btreeset![
                FileTreeNode::File("fake-flag.txt".to_string()),
                FileTreeNode::File("fake-flag_000.txt".to_string()),
                FileTreeNode::File("fake-flag_001.txt".to_string()),
                FileTreeNode::File("fake-flag_002.txt".to_string()),
                FileTreeNode::File("fake-flag_003.txt".to_string()),
            ],
        );

        let filenames = [
            "--fake-flag.txt",
            "fake-flag.txt",
            "------fake-flag.txt",
            " fake-flag.txt",
            "\tfake-flag.txt",
        ];
        f(
            &["unf", ".", "-rf"],
            filenames_to_file_tree(&filenames),
            btreeset![
                FileTreeNode::File("fake-flag.txt".to_string()),
                FileTreeNode::File("fake-flag_000.txt".to_string()),
                FileTreeNode::File("fake-flag_001.txt".to_string()),
                FileTreeNode::File("fake-flag_002.txt".to_string()),
                FileTreeNode::File("fake-flag_003.txt".to_string()),
            ],
        );
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

fn read_children_names<FS: GenFS>(fs: &FS, cwd: &Path, dir: &Path) -> Result<Vec<OsString>> {
    let mut children_names = fs
        .read_dir(cwd.join(dir))?
        .map(|result_ent| result_ent.map(|ent| ent.file_name()))
        .collect::<std::io::Result<Vec<OsString>>>()?;
    children_names.sort();
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
            prompt_default(msg, false)
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
        if !prompt_default(msg, false) {
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
fn path_exists<FS: GenFS>(fs: &FS, cwd: &Path, path: &Path) -> bool {
    fs.metadata(cwd.join(path)).is_ok()
}

fn unixize_paths<FS: GenFS>(fs: &FS, cwd: &Path, paths: &[PathBuf], flags: Flags) -> Result<()> {
    for path in paths {
        unixize_path(fs, cwd, &path, flags)?;
    }
    Ok(())
}

fn load_mem_fs_insert(fs: &rsfs::mem::FS, path: &Path) -> Result<()> {
    if path.is_dir() {
        fs.create_dir(path)?;
        for ent in path.read_dir()? {
            let path = ent?.path();
            load_mem_fs_insert(fs, &path)?;
        }
    } else {
        fs.create_file(path)?;
    }
    Ok(())
}

fn load_mem_fs(paths: &[PathBuf]) -> Result<rsfs::mem::FS> {
    let fs = rsfs::mem::FS::new();

    for path in paths {
        let path = path.canonicalize()?;
        if let Some(parent) = path.parent() {
            fs.create_dir_all(parent)?;
        }
        load_mem_fs_insert(&fs, &path)?;
    }

    Ok(fs)
}

fn main_opts(opts: Opts) -> Result<()> {
    let cwd = std::env::current_dir()?;

    if opts.flags.dry_run {
        let fs = load_mem_fs(&opts.paths)?;
        unixize_paths(&fs, &cwd, &opts.paths, opts.flags)
    } else {
        let fs = rsfs::disk::FS;
        unixize_paths(&fs, &cwd, &opts.paths, opts.flags)
    }
}

fn try_main() -> Result<()> {
    main_opts(Opts::from_args())
}

fn main() {
    if let Err(err) = try_main() {
        eprintln!("unf: error: {}", err);
        std::process::exit(1);
    }
}
