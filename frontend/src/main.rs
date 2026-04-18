use dioxus::prelude::*;
use aetheris_frontend::App;

fn main() {
    console_error_panic_hook::set_once();
    launch(App);
}
