//! Load a file tree into an in-memory filesystem

use crate::Result;

use std::path::Path;
use std::path::PathBuf;

use rsfs::GenFS;

/// Load file or directory from `path` into `fs`, including all children.
fn load_insert(fs: &rsfs::mem::FS, path: &Path) -> Result<()> {
    if path.is_dir() {
        fs.create_dir(path)?;

        // Load children
        for ent in path.read_dir()? {
            let path = ent?.path();
            load_insert(fs, &path)?;
        }
    } else {
        fs.create_file(path)?;
    }
    Ok(())
}

/// Load the parts of the physical filesystem referenced by `paths` into a new
/// in-memory filesystem. After the `paths` are canonicalized, all leading
/// components and children will be created in the memory filesystem.
///
/// ## Example
///
/// A file structure
/// ```text
/// /tmp
/// ├── a
/// │   └── b
/// ├── c
/// └── foo
///     ├── bar
///     └── baz
///         └── a
/// ```
/// with `paths` = `["a", "foo/baz"]` and a working directory of `/tmp` would
/// return an in-memory filesystem:
/// ```text
/// /tmp
/// ├── a
/// │   └── b
/// └── foo
///     └── baz
///         └── a
/// ```
pub fn load(paths: &[PathBuf]) -> Result<rsfs::mem::FS> {
    let fs = rsfs::mem::FS::new();

    for path in paths {
        // Create all parents of canonicalized path
        let path = path.canonicalize()?;
        if let Some(parent) = path.parent() {
            fs.create_dir_all(parent)?;
        }

        // Recursively create path and children
        load_insert(&fs, &path)?;
    }

    Ok(fs)
}
