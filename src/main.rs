#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate clap;
#[macro_use]
extern crate failure;
extern crate promptly;
extern crate regex;
#[macro_use]
#[cfg(test)]
extern crate maplit;

use promptly::prompt_default;
use regex::Regex;
use std::fs::read_dir;
use std::path::{Path, PathBuf};

type Result<T> = std::result::Result<T, Box<std::error::Error>>;

// Struct representing a filename that can be split, modified, and
// merged back into a filename string
#[derive(PartialEq, Debug)]
struct FilenameParts {
    stem: String, // From the beginning of the filename to the final dot before the extension
    num: Option<usize>, // The zero-padded collision-resolving number
    ext: Option<String>, // The file extension, not including the dot
}

const FILENAME_NUM_DIGITS: usize = 3;

#[cfg(test)]
mod tests {
    extern crate tempdir;

    use std::collections::BTreeSet;
    use std::fs::File;
    use tempdir::TempDir;

    use super::*;

    // Representation of a virtual file tree used for test cases
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
        assert_eq!(f("🤔😀😃😄😁😆😅emojis.txt"), "emojis.txt");
        assert_eq!(f("Game (Not Pirated 😉).rar"), "Game_Not_Pirated.rar");
        assert_eq!(f("--fake-flag"), "fake-flag");
    }

    #[test]
    fn test_split_filename() {
        assert_eq!(
            split_filename("a"),
            FilenameParts {
                stem: "a".to_string(),
                num: None,
                ext: None,
            }
        );
        assert_eq!(
            split_filename("a."),
            FilenameParts {
                stem: "a".to_string(),
                num: None,
                ext: Some("".to_string()),
            }
        );
        assert_eq!(
            split_filename(".a"),
            FilenameParts {
                stem: "".to_string(),
                num: None,
                ext: Some("a".to_string()),
            }
        );
        assert_eq!(
            split_filename("a_0000"),
            FilenameParts {
                stem: "a_0000".to_string(),
                num: None,
                ext: None,
            }
        );
        assert_eq!(
            split_filename("a_137"),
            FilenameParts {
                stem: "a".to_string(),
                num: Some(137),
                ext: None,
            }
        );
        assert_eq!(
            split_filename("a_000.txt"),
            FilenameParts {
                stem: "a".to_string(),
                num: Some(0),
                ext: Some("txt".to_string()),
            }
        );
        assert_eq!(
            split_filename("a____000.txt"),
            FilenameParts {
                stem: "a___".to_string(),
                num: Some(0),
                ext: Some("txt".to_string()),
            }
        );
        assert_eq!(
            split_filename(".x._._._222.txt"),
            FilenameParts {
                stem: ".x._._.".to_string(),
                num: Some(222),
                ext: Some("txt".to_string()),
            }
        );
    }

    #[test]
    fn test_merge_filename() {
        assert_eq!(
            "a",
            merge_filename(&FilenameParts {
                stem: "a".to_string(),
                num: None,
                ext: None,
            })
        );
        assert_eq!(
            "a.",
            merge_filename(&FilenameParts {
                stem: "a".to_string(),
                num: None,
                ext: Some("".to_string()),
            })
        );
        assert_eq!(
            ".a",
            merge_filename(&FilenameParts {
                stem: "".to_string(),
                num: None,
                ext: Some("a".to_string()),
            })
        );
        assert_eq!(
            "a_0000",
            merge_filename(&FilenameParts {
                stem: "a_0000".to_string(),
                num: None,
                ext: None,
            })
        );
        assert_eq!(
            "a_137",
            merge_filename(&FilenameParts {
                stem: "a".to_string(),
                num: Some(137),
                ext: None,
            })
        );
        assert_eq!(
            "a_000.txt",
            merge_filename(&FilenameParts {
                stem: "a".to_string(),
                num: Some(0),
                ext: Some("txt".to_string()),
            })
        );
        assert_eq!(
            "a____000.txt",
            merge_filename(&FilenameParts {
                stem: "a___".to_string(),
                num: Some(0),
                ext: Some("txt".to_string()),
            })
        );
        assert_eq!(
            ".x._._._222.txt",
            merge_filename(&FilenameParts {
                stem: ".x._._.".to_string(),
                num: Some(222),
                ext: Some("txt".to_string()),
            })
        );
    }

    // Scan the file structure in a path to `FileTree`
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

    // Actually create the file structure represented by a `FileTree`
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

    // Create the file structure represented by `FileTree` in a
    // temporary directory and return its path
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
    fn test_try_main_with_args() {
        let mut app = make_clap_app();

        // Helper function to create a specified file structure and
        // run `unf` with the specified args. It then asserts that the
        // resulting file structure matches the expected result.
        let mut f = |args: &[&str], tree: FileTree, expected: FileTree| {
            let path = create_tree_tmp(tree);
            std::env::set_current_dir(&path).unwrap();

            let args = app.get_matches_from_safe_borrow(args).unwrap();
            try_main_with_args(args).unwrap();

            let result = scan_tree(&path);
            assert_eq!(expected, result);
        };

        let s = "🤔😀😃😄😁😆😅emojis.txt";
        f(
            &["unf", "-f", s],
            btreeset![FileTreeNode::File(s.to_string())],
            btreeset![FileTreeNode::File("emojis.txt".to_string())],
        );

        let s = "Game (Not Pirated 😉).rar";
        f(
            &["unf", "-f", s],
            btreeset![FileTreeNode::File(s.to_string())],
            btreeset![FileTreeNode::File("Game_Not_Pirated.rar".to_string())],
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
    }
}

// Clean up a string representing a filename, replacing
// unix-unfriendly characters (like spaces, parentheses, etc.) See the
// unit tests for examples.
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
    // Remove leading and trailing underscores and hyphens
    let s = s.trim_matches(|c| c == '_' || c == '-');
    s.to_string()
}

// Use clap crate to create argument parser
fn make_clap_app() -> clap::App<'static, 'static> {
    app_from_crate!().args_from_usage(
        "<PATH>... 'The paths of filenames to unixize'
             -r --recursive 'Recursively unixize filenames in directories. If \
                             some of the specified paths are directories, unf \
                             will operate recursively on their contents'
             -f --force 'Do not interactively prompt to rename each file'",
    )
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
        .ok_or_else(|| format_err!("path '{}' has no basename", path.display()))?
        .to_string_lossy();
    let new_basename = unixize_filename_str(basename);

    let stat = std::fs::metadata(path)?;
    let is_dir = stat.is_dir();
    let should_prompt = !args.is_present("force");

    // Determine whether to recurse, possibly by prompting the user
    let recurse = args.is_present("recursive")
        && is_dir
        && (!should_prompt || {
            let msg = format!("descend into directory '{}'?", path.display());
            prompt_default(msg, false)
        });

    if recurse {
        for ent in read_dir(path)? {
            unixize_filename(&ent?.path(), args)?;
        }
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

// Split, modify, and re-merge filename to increment the
// collision-resolving number, or create it if non-existent
fn inc_filename_num(filename: &str) -> String {
    let FilenameParts { stem, num, ext } = split_filename(filename);
    let num = match num {
        Some(val) => Some(val + 1),
        None => Some(0),
    };
    merge_filename(&FilenameParts { stem, num, ext })
}

// Check if the target path can be written to without clobbering an
// existing file. If it can't, change it to a unique name. Note that
// this function requires that the filename is non-empty and valid
// UTF-8.
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

fn split_filename(filename: &str) -> FilenameParts {
    let mut it = filename.rsplitn(2, '.');
    let ext = it.next().expect("tried to split empty filename");
    let maybe_stem_num = it.next();

    // Set the stem-num combination to the extension if the iterator
    // said it was `None`. This is such that only the content after
    // the final dot is considered the extension, but extension-less
    // files are properly handled.
    let (stem_num, ext) = match maybe_stem_num {
        Some(stem_num) => (stem_num, Some(ext.to_string())),
        None => (ext, None),
    };

    // Hack to get an iterator over the last `FILENAME_NUM_DIGITS + 1`
    // characters of the stem-num combination. For files that have the
    // collision-resolving number, this is that prefixed with an
    // underscore.
    let num_it = stem_num
        .chars()
        .rev()
        .take(FILENAME_NUM_DIGITS + 1)
        .collect::<Vec<_>>();
    let mut num_it = num_it.iter().rev();

    // Determine if the filename has a collision-resolving number and
    // parse it
    let num = if num_it.next() == Some(&'_') && num_it.len() == FILENAME_NUM_DIGITS {
        num_it.collect::<String>().parse::<usize>().ok()
    } else {
        None
    };

    // Split the stem from the stem-num combination
    let stem = if num.is_some() {
        stem_num
            .chars()
            .take(stem_num.len() - FILENAME_NUM_DIGITS - 1)
            .collect()
    } else {
        stem_num.to_string()
    };

    FilenameParts { stem, num, ext }
}

// Format the collision-resolving number of a filename to a
// zero-padded string
fn format_filename_num(num: usize) -> String {
    format!("{:0width$}", num, width = FILENAME_NUM_DIGITS)
}

fn merge_filename(parts: &FilenameParts) -> String {
    let mut s = String::new();
    s.push_str(&parts.stem);
    if let Some(num) = parts.num {
        s.push('_');
        s.push_str(&format_filename_num(num));
    }
    if let Some(ref ext) = parts.ext {
        s.push('.');
        s.push_str(ext);
    }
    s
}

fn try_main_with_args(args: clap::ArgMatches<'static>) -> Result<()> {
    for path in args.values_of("PATH").expect("no arguments").map(Path::new) {
        unixize_filename(path, &args)?;
    }

    Ok(())
}

fn try_main() -> Result<()> {
    let args = make_clap_app().get_matches();

    try_main_with_args(args)
}

fn main() {
    if let Err(err) = try_main() {
        eprintln!("unf: error: {}", err);
        std::process::exit(1);
    }
}
