#[macro_use]
extern crate clap;

fn main() {
    let matches = app_from_crate!().get_matches();
    println!("{:?}", matches);
}
