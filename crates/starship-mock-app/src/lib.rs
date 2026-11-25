pub mod bridge;
pub mod faults;
pub mod scenario;

pub use bridge::{build_bridge, serve};
pub use faults::{FaultRuntime, ProcessFaults, RecordingFaults, TerminalFault};
pub use scenario::{FailureMode, Scenario, ToolScenario};

use std::io::{BufReader, Error};

/// Serves the scenario over standard input and output for headless MCP clients.
///
/// # Errors
/// Returns an I/O error when a pipe cannot be read or written.
pub fn serve_stdio(scenario: &Scenario) -> Result<(), Error> {
    let bridge = build_bridge(scenario);
    let faults = ProcessFaults::new(scenario.oom_bytes);
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    serve(&bridge, scenario, &faults, stdin.lock(), stdout.lock())
}

/// Publishes the manifest and serves the scenario over its Unix domain socket.
///
/// # Errors
/// Returns an I/O error when the socket cannot be bound or a client fails.
pub fn serve_socket(scenario: &Scenario) -> Result<(), Error> {
    let bridge = build_bridge(scenario);
    let faults = ProcessFaults::new(scenario.oom_bytes);
    let server = bridge.serve_socket()?;
    eprintln!("mock app listening at {}", server.socket_path().display());

    for connection in server.listener().incoming() {
        match connection {
            Ok(stream) => {
                let reader = BufReader::new(stream.try_clone()?);
                if let Err(error) = serve(&bridge, scenario, &faults, reader, stream) {
                    eprintln!("mock app client error: {error}");
                }
            }
            Err(error) => eprintln!("mock app connection error: {error}"),
        }
    }
    Ok(())
}
