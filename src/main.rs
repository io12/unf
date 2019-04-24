#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate clap;
extern crate regex;

use regex::Regex;
use std::io::Error;
use std::path::Path;

fn unixize_filename_str(fname: &str) -> String {
    lazy_static! {
        static ref RE_INVAL_CHR: Regex = Regex::new("[^a-zA-Z0-9._-]").unwrap();
        static ref RE_UND_DUP: Regex = Regex::new("_+").unwrap();
    }
    // Replace all invalid characters with underscores
    let s = RE_INVAL_CHR.replace_all(fname, "_");
    // Remove duplicate underscores
    let s = RE_UND_DUP.replace_all(&s, "_");
    // Remove leading and trailing underscores
    let s = s.trim_matches('_');
    s.to_string()
}

fn main() -> Result<(), Error> {
    let matches = app_from_crate!()
        // TODO: make usage more descriptive
        .args_from_usage(
            "<PATH>... 'The paths of filenames to unixize'
             -r --recursive 'Recursively unixize filenames in directories'
             -q --quiet 'Do not write to stdout'
             -d --dryrun 'Do not rename files, only print names'",
        )
        .get_matches();

    let cwd = std::env::current_dir()?;

    // Here unwrap() is safe because PATH is a required argument
    for path in matches.values_of("PATH").unwrap().map(Path::new) {
        let parent = path.parent().unwrap_or(&cwd);
        let basename = &path.file_name().unwrap().to_string_lossy();
        let new_basename = unixize_filename_str(basename);
        let new_path = parent.join(new_basename);
        println!("{:?}", new_path);
    }

    Ok(())
}
