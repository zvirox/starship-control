use std::sync::Mutex;
use std::thread;
use std::time::Duration;

pub const CRASH_EXIT_CODE: i32 = 70;
pub const OOM_EXIT_CODE: i32 = 137;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalFault {
    Crash,
    Hang,
    Oom,
}

pub trait FaultRuntime: Send + Sync {
    /// Applies a fatal fault. Returns an error when the runtime cannot terminate.
    ///
    /// # Errors
    /// Test runtimes return a marker error instead of terminating the process.
    fn trigger(&self, fault: TerminalFault) -> Result<(), String>;
}

pub struct ProcessFaults {
    oom_bytes: usize,
}

impl ProcessFaults {
    #[must_use]
    pub fn new(oom_bytes: usize) -> Self {
        Self { oom_bytes }
    }
}

impl FaultRuntime for ProcessFaults {
    fn trigger(&self, fault: TerminalFault) -> Result<(), String> {
        match fault {
            TerminalFault::Crash => {
                eprintln!("mock app: simulated crash during tool call");
                std::process::exit(CRASH_EXIT_CODE);
            }
            TerminalFault::Hang => loop {
                thread::sleep(Duration::from_secs(3600));
            },
            TerminalFault::Oom => {
                let mut allocated = Vec::new();
                let _ = allocated.try_reserve_exact(self.oom_bytes);
                allocated.fill(1);
                eprintln!("mock app: simulated OOM kill at {} bytes", allocated.len());
                std::process::exit(OOM_EXIT_CODE);
            }
        }
    }
}

#[derive(Debug, Default)]
pub struct RecordingFaults {
    triggered: Mutex<Vec<TerminalFault>>,
}

impl RecordingFaults {
    #[must_use]
    pub fn triggered(&self) -> Vec<TerminalFault> {
        self.triggered
            .lock()
            .map_or_else(|_| Vec::new(), |faults| faults.clone())
    }
}

impl FaultRuntime for RecordingFaults {
    fn trigger(&self, fault: TerminalFault) -> Result<(), String> {
        if let Ok(mut triggered) = self.triggered.lock() {
            triggered.push(fault);
        }
        Err(format!("simulated {fault:?}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recording_runtime_records_without_terminating() {
        let runtime = RecordingFaults::default();
        assert!(runtime.trigger(TerminalFault::Crash).is_err());
        assert_eq!(runtime.triggered(), vec![TerminalFault::Crash]);
    }
}
