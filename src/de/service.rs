use ark::service::{service, RunLevel, spawn_child_service};

#[service(name = "ui_shell")]
fn ui_shell_service() {
	spawn_child_service!(taskbar_service);
}
