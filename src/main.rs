use std::env;

mod editor;

fn main() {
     editor::Editor::new(env::args_os().skip(1)).unwrap().run().unwrap();
}
