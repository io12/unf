#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate clap;
extern crate regex;

use regex::Regex;

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

fn main() {
    let matches = app_from_crate!()
        // TODO: make usage more descriptive
        .args_from_usage(
            "<PATH>... 'The paths of filenames to unixize'
             -r --recursive 'Recursively unixize filenames in directories'
             -q --quiet 'Do not write to stdout'
             -d --dryrun 'Do not rename files, only print names'",
        )
        .get_matches();

    // Here unwrap() is safe because PATH is a required argument
    for path in matches.values_of("PATH").unwrap() {
        let new = unixize_filename_str(path);
        println!("{}", new);
    }
}
