#[macro_use]
extern crate clap;

fn main() {
    let matches = app_from_crate!()
        // TODO: make usage more descriptive
        .args_from_usage(
            "[PATH]... 'The paths of filenames to unixize'
             -r --recursive 'Recursively unixize filenames in directories'
             -q --quiet 'Do not write to stdout'",
        )
        .get_matches();
}
