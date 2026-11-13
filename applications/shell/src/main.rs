use ark::application::contract::{default_func, export_func};

#[export_func]
fn run_command(command_name: &'static str) {
    // ...
}

#[default_func]
fn entry_point() {
    // ...
}
