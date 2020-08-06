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
use std::fs::read_dir;
use std::path::Path;
use std::path::PathBuf;

use deunicode::deunicode;
use promptly::prompt_default;
use regex::Regex;
use structopt::StructOpt;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[cfg(test)]
mod tests {
    extern crate tempdir;

    use std::collections::BTreeSet;
    use std::fs::File;

    use tempdir::TempDir;

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
    fn scan_tree(path: &Path) -> FileTree {
        let mut tree = FileTree::new();
        for ent in read_dir(path).unwrap() {
            let ent = ent.unwrap();
            let is_dir = ent.file_type().unwrap().is_dir();
            let filename = ent.file_name().into_string().unwrap();
            let ent = if is_dir {
                FileTreeNode::Dir(filename, scan_tree(&ent.path()))
            } else {
                FileTreeNode::File(filename)
            };
            tree.insert(ent);
        }
        tree
    }

    /// Actually create the file structure represented by a `FileTree`
    fn create_tree(tree: FileTree, path: &Path) {
        for ent in tree {
            match ent {
                FileTreeNode::File(name) => {
                    File::create(path.join(name)).unwrap();
                }
                FileTreeNode::Dir(name, ents) => {
                    let path = path.join(name);
                    std::fs::create_dir(&path).unwrap();
                    create_tree(ents, &path);
                }
            }
        }
    }

    /// Create the file structure represented by `FileTree` in a
    /// temporary directory and return its path
    fn create_tree_tmp(tree: FileTree) -> PathBuf {
        let path = TempDir::new("").unwrap().into_path();
        create_tree(tree, &path);
        path
    }

    #[test]
    fn test_resolve_collision() {
        let tmp_dir = TempDir::new("").unwrap().into_path();

        // Helper function taking a collider filename returning a
        // string representing the resolved collision
        let f = |filename: &str| -> String {
            let path = tmp_dir.join(filename);
            File::create(&path).unwrap();

            resolve_collision(&path)
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

            let result = scan_tree(&path);
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

/// Like `unixize_filename()`, but only operate on children of `path`
fn unixize_children(path: &Path, flags: Flags) -> Result<()> {
    for ent in read_dir(path)? {
        unixize_filename(&ent?.path(), flags)?;
    }
    Ok(())
}

/// Unixize the filename(s) specified by a path, according to the
/// supplied arguments
fn unixize_filename(path: &Path, flags: Flags) -> Result<()> {
    lazy_static! {
        static ref CWD: PathBuf = std::env::current_dir().unwrap();
    }

    let parent = path.parent().unwrap_or(&CWD);
    let basename = &path.file_name().map(OsStr::to_string_lossy);
    let basename = match basename {
        Some(s) => s,
        // If the path has no basename (for example, if it's `.` or `..`), only
        // unixize children
        None => return unixize_children(path, flags),
    };
    let new_basename = unixize_filename_str(basename);

    let stat = std::fs::metadata(path)?;
    let is_dir = stat.is_dir();
    let should_prompt = !flags.force;

    // Determine whether to recurse, possibly by prompting the user
    let recurse = flags.recursive
        && is_dir
        && (!should_prompt || {
            let msg = format!("descend into directory '{}'?", path.display());
            prompt_default(msg, false)
        });

    if recurse {
        unixize_children(path, flags)?;
    }

    // Skip files that already have unix-friendly names; this is done
    // after recursive handling because unix-friendly directory names
    // might have non-unix-friendly filenames inside
    if basename == &new_basename {
        return Ok(());
    }

    let new_path = parent.join(new_basename);
    let new_path = resolve_collision(&new_path);
    let msg = format!("rename '{}' -> '{}'", path.display(), new_path.display());
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

    std::fs::rename(path, new_path)?;
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
fn resolve_collision(path: &Path) -> PathBuf {
    if path.exists() {
        let filename = path
            .file_name()
            .expect("filename is empty")
            .to_str()
            .expect("filename is not valid UTF-8");
        let filename = inc_filename_num(filename);
        let path = path.with_file_name(filename);

        // Recursively resolve the new filename. This is how the
        // collision-resolving number is incremented.
        resolve_collision(&path)
    } else {
        // File does not exist; we're done!
        path.to_path_buf()
    }
}

fn main_opts(opts: Opts) -> Result<()> {
    for path in opts.paths {
        unixize_filename(&path, opts.flags)?;
    }

    Ok(())
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
