use starship_mock_app::{Scenario, serve_socket, serve_stdio};
use std::env;

fn scenario_path(arguments: &[String]) -> Option<&str> {
    arguments
        .iter()
        .position(|argument| argument == "--scenario")
        .and_then(|position| arguments.get(position + 1))
        .map(String::as_str)
}

fn run() -> Result<(), String> {
    let arguments: Vec<String> = env::args().skip(1).collect();
    let scenario = match scenario_path(&arguments) {
        Some(path) => Scenario::from_file(path)?,
        None => Scenario::from_env()?,
    };

    if arguments.iter().any(|argument| argument == "--mcp") {
        serve_stdio(&scenario).map_err(|error| format!("stdio bridge failed: {error}"))
    } else {
        serve_socket(&scenario).map_err(|error| format!("socket bridge failed: {error}"))
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("starship-mock-app: {error}");
        std::process::exit(1);
    }
}
