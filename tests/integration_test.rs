use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::path::PathBuf;

use tempfile::TempDir;
use walkdir::WalkDir;

fn run_unf<B, P, PBUF, S, PS>(
    current_dir: P,
    args: &[S],
    stdin: B,
    expected_stdout: B,
    expected_stderr: B,
    before_paths: PS,
    expected_after_paths: PS,
) where
    B: AsRef<[u8]>,
    P: AsRef<Path>,
    PBUF: Into<PathBuf>,
    S: AsRef<OsStr>,
    PS: IntoIterator<Item = PBUF>,
{
    let root = TempDir::new().unwrap();
    let root = root.path();

    for path in before_paths {
        let path = path.into();
        let path = root.join(path);

        // Create all parents
        let result = fs::create_dir_all(path.parent().unwrap());
        match result {
            Ok(()) => {}
            Err(err) if err.kind() == io::ErrorKind::AlreadyExists => {}
            Err(err) => panic!("Failed creating parents: {}", err),
        }

        // Create path
        let is_file = path.file_name().unwrap().as_bytes().contains(&b'.');
        if is_file {
            fs::File::create(path).unwrap();
        } else {
            fs::create_dir(path).unwrap();
        }
    }

    assert_cmd::Command::cargo_bin("unf")
        .unwrap()
        .current_dir(root.join(current_dir))
        .args(args)
        .write_stdin(stdin.as_ref())
        .assert()
        .success()
        .stdout(Box::leak(expected_stdout.as_ref().to_vec().into_boxed_slice()) as &[u8])
        .stderr(Box::leak(expected_stderr.as_ref().to_vec().into_boxed_slice()) as &[u8]);

    let expected_after_paths = expected_after_paths
        .into_iter()
        .map(|path| path.into())
        .collect::<BTreeSet<PathBuf>>();
    let actual_after_paths = WalkDir::new(root)
        .into_iter()
        .map(|ent| {
            ent.unwrap()
                .into_path()
                .strip_prefix(root)
                .unwrap()
                .to_path_buf()
        })
        .filter(|path| !path.as_os_str().is_empty())
        .collect::<BTreeSet<PathBuf>>();
    assert_eq!(expected_after_paths, actual_after_paths);
}

#[test]
fn integration_test() {
    run_unf(
        ".",
        &["-f", "🤔😀😃😄😁😆😅emojis.txt"],
        "",
        "rename '🤔😀😃😄😁😆😅emojis.txt' -> 'thinking_grinning_smiley_smile_grin_laughing_sweat_smile_emojis.txt'\n",
        "",
        &["🤔😀😃😄😁😆😅emojis.txt"],
        &["thinking_grinning_smiley_smile_grin_laughing_sweat_smile_emojis.txt"],
    );
    run_unf(
        ".",
        &["-f", "Game (Not Pirated 😉).rar"],
        "",
        "rename 'Game (Not Pirated 😉).rar' -> 'Game_Not_Pirated_wink.rar'\n",
        "",
        &["Game (Not Pirated 😉).rar"],
        &["Game_Not_Pirated_wink.rar"],
    );
    run_unf(
        ".",
        &["-rf", "My Files/", "My Folder"],
        "",
        concat!(
            "rename 'My Files/Another Cool Photo.JPG' -> 'My Files/Another_Cool_Photo.JPG'\n",
            "rename 'My Files/Cool Photo.JPG' -> 'My Files/Cool_Photo.JPG'\n",
            "rename 'My Files/Passwords :) .txt' -> 'My Files/Passwords.txt'\n",
            "rename 'My Files/Wow Cool Photo.JPG' -> 'My Files/Wow_Cool_Photo.JPG'\n",
            "rename 'My Files/' -> 'My_Files'\n",
            "rename 'My Folder' -> 'My_Folder'\n",
        ),
        "",
        &[
            "My Folder",
            "My Files",
            "My Files/Passwords :) .txt",
            "My Files/Another Cool Photo.JPG",
            "My Files/Wow Cool Photo.JPG",
            "My Files/Cool Photo.JPG",
        ],
        &[
            "My_Folder",
            "My_Files",
            "My_Files/Passwords.txt",
            "My_Files/Another_Cool_Photo.JPG",
            "My_Files/Wow_Cool_Photo.JPG",
            "My_Files/Cool_Photo.JPG",
        ],
    );
    run_unf(
        ".",
        &["-rd", "My Files/", "My Folder"],
        "",
        concat!(
            "would rename 'My Files/Another Cool Photo.JPG' -> 'My Files/Another_Cool_Photo.JPG'\n",
            "would rename 'My Files/Cool Photo.JPG' -> 'My Files/Cool_Photo.JPG'\n",
            "would rename 'My Files/Passwords :) .txt' -> 'My Files/Passwords.txt'\n",
            "would rename 'My Files/Wow Cool Photo.JPG' -> 'My Files/Wow_Cool_Photo.JPG'\n",
            "would rename 'My Files/' -> 'My_Files'\n",
            "would rename 'My Folder' -> 'My_Folder'\n",
        ),
        "",
        &[
            "My Folder",
            "My Files",
            "My Files/Passwords :) .txt",
            "My Files/Another Cool Photo.JPG",
            "My Files/Wow Cool Photo.JPG",
            "My Files/Cool Photo.JPG",
        ],
        &[
            "My Folder",
            "My Files",
            "My Files/Passwords :) .txt",
            "My Files/Another Cool Photo.JPG",
            "My Files/Wow Cool Photo.JPG",
            "My Files/Cool Photo.JPG",
        ],
    );
    run_unf(
        ".",
        &[
            "-f",
            "--",
            "--fake-flag.txt",
            "fake-flag.txt",
            "------fake-flag.txt",
            " fake-flag.txt",
            "\tfake-flag.txt",
        ],
        "",
        concat!(
            "rename '--fake-flag.txt' -> 'fake-flag_000.txt'\n",
            "rename '------fake-flag.txt' -> 'fake-flag_001.txt'\n",
            "rename ' fake-flag.txt' -> 'fake-flag_002.txt'\n",
            "rename '\tfake-flag.txt' -> 'fake-flag_003.txt'\n",
        ),
        "",
        &[
            "--fake-flag.txt",
            "fake-flag.txt",
            "------fake-flag.txt",
            " fake-flag.txt",
            "\tfake-flag.txt",
        ],
        &[
            "fake-flag.txt",
            "fake-flag_000.txt",
            "fake-flag_001.txt",
            "fake-flag_002.txt",
            "fake-flag_003.txt",
        ],
    );
}
