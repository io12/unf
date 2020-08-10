//! Load a file tree into an in-memory filesystem

use crate::Result;

use std::path::Path;

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
pub fn load<P: AsRef<Path>>(paths: &[P]) -> Result<rsfs::mem::FS> {
    let fs = rsfs::mem::FS::new();

    for path in paths {
        let path = path.as_ref();

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

#[cfg(test)]
mod tests {
    use super::*;

    use crate::path_exists;

    #[test]
    fn test_load() {
        let tmp = tempfile::TempDir::new().unwrap();
        let tmp = tmp.path();

        // Create file tree
        std::fs::create_dir_all(tmp.join("a")).unwrap();
        std::fs::File::create(tmp.join("a/b")).unwrap();
        std::fs::File::create(tmp.join("c")).unwrap();
        std::fs::create_dir_all(tmp.join("foo/baz")).unwrap();
        std::fs::File::create(tmp.join("foo/bar")).unwrap();
        std::fs::File::create(tmp.join("foo/baz/a")).unwrap();

        // Load file tree
        std::env::set_current_dir(tmp).unwrap();
        let fs = load(&["a", "foo/baz"]).unwrap();

        // Check in-memory filesystem

        assert!(!path_exists(&fs, tmp, "c"));
        assert!(!path_exists(&fs, tmp, "foo/bar"));

        assert!(path_exists(&fs, tmp, "a"));
        assert!(path_exists(&fs, tmp, "a/b"));
        assert!(path_exists(&fs, tmp, "foo/baz"));
        assert!(path_exists(&fs, tmp, "foo/baz/a"));
    }
}
