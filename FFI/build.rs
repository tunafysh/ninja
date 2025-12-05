use std::fs;

fn main() {
    fs::create_dir_all("../include").expect("Could not create include directory");
    cbindgen::generate(".")
        .expect("Could not generate header")
        .write_to_file("../include/ninja.h");
}
