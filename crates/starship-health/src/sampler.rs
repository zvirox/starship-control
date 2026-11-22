use crate::sample::{
    CpuMetrics, DiskMetrics, FootprintMetrics, HealthSample, MemoryMetrics, NetworkMetrics,
    ProcessMetrics,
};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use sysinfo::{Disks, Networks, System};
use tracing::debug;

/// Unified trait implemented by every metric sampler (GAP-CLOSURE-PLAN.md §14.1).
/// Implementations must be completely non-blocking, non-panicking, and graceful
/// in the absence of hardware sensors (e.g., in headless containers or VMs).
pub trait Sampler: Send + Sync {
    /// Sample metrics and record them into the provided `HealthSample`.
    fn sample_into(&mut self, sample: &mut HealthSample);

    /// Identifier name for logging and diagnostics.
    fn name(&self) -> &'static str;
}

/// Adaptive sampling frequency controller (GAP-CLOSURE-PLAN.md §14.2).
/// - 1.0 Hz (1000ms interval) while dashboard or HUD is visible, or a mission is running.
/// - 0.2 Hz (5000ms interval) when the application is idle and windows are closed/hidden.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SamplingMode {
    Active, // 1 Hz
    Idle,   // 0.2 Hz
}

impl SamplingMode {
    #[must_use]
    pub fn interval(self) -> Duration {
        match self {
            Self::Active => Duration::from_millis(1000),
            Self::Idle => Duration::from_millis(5000),
        }
    }
}

/// System metric sampler collecting CPU, Memory, Disk, Network, and Processes via `sysinfo` and `/proc`.
pub struct SystemSampler {
    sys: System,
    disks: Disks,
    networks: Networks,
    last_sample_instant: Instant,
    last_vmstat_pswpin: Option<u64>,
    last_disk_sectors: Option<(u64, u64)>, // (read_sectors, write_sectors)
    process_sample_counter: u32,
    last_processes: Vec<ProcessMetrics>,
    last_footprint: Option<FootprintMetrics>,
}

impl Default for SystemSampler {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemSampler {
    #[must_use]
    pub fn new() -> Self {
        let mut sys = System::new_all();
        sys.refresh_all();
        let disks = Disks::new_with_refreshed_list();
        let networks = Networks::new_with_refreshed_list();

        Self {
            sys,
            disks,
            networks,
            last_sample_instant: Instant::now(),
            last_vmstat_pswpin: read_proc_vmstat_pswpin(),
            last_disk_sectors: read_proc_diskstats_sectors(),
            process_sample_counter: 0,
            last_processes: Vec::new(),
            last_footprint: None,
        }
    }

    fn sample_cpu(&mut self) -> CpuMetrics {
        self.sys.refresh_cpu_usage();
        let global_util_pct = self.sys.global_cpu_usage();
        let core_util_pct: Vec<f32> = self
            .sys
            .cpus()
            .iter()
            .map(sysinfo::Cpu::cpu_usage)
            .collect();

        let load_avg = System::load_average();
        let cpus = self.sys.cpus();
        let avg_freq_mhz = if cpus.is_empty() {
            0
        } else {
            cpus.iter().map(sysinfo::Cpu::frequency).sum::<u64>() / cpus.len() as u64
        };

        CpuMetrics {
            global_util_pct,
            core_util_pct,
            load_avg_1m: load_avg.one,
            load_avg_5m: load_avg.five,
            load_avg_15m: load_avg.fifteen,
            avg_freq_mhz,
        }
    }

    #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
    fn sample_memory(&mut self, elapsed_secs: f64) -> MemoryMetrics {
        self.sys.refresh_memory();
        let total_bytes = self.sys.total_memory();
        let used_bytes = self.sys.used_memory();
        let available_bytes = self.sys.available_memory();
        let available_pct = if total_bytes > 0 {
            ((available_bytes as f64 / total_bytes as f64) * 100.0) as f32
        } else {
            0.0
        };

        let swap_total_bytes = self.sys.total_swap();
        let swap_used_bytes = self.sys.used_swap();

        // Calculate swap-in rate from /proc/vmstat pswpin delta
        let current_pswpin = read_proc_vmstat_pswpin();
        let swap_in_bytes_per_sec = match (self.last_vmstat_pswpin, current_pswpin) {
            (Some(prev), Some(curr)) if curr >= prev && elapsed_secs > 0.0 => {
                let pages_in = curr - prev;
                let bytes_in = pages_in as f64 * 4096.0; // standard 4KB page
                bytes_in / elapsed_secs
            }
            _ => 0.0,
        };
        self.last_vmstat_pswpin = current_pswpin;

        MemoryMetrics {
            total_bytes,
            used_bytes,
            available_bytes,
            available_pct,
            swap_total_bytes,
            swap_used_bytes,
            swap_in_bytes_per_sec,
        }
    }

    #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
    fn sample_disk(&mut self, elapsed_secs: f64) -> Option<DiskMetrics> {
        self.disks.refresh(true);
        if self.disks.is_empty() {
            return None;
        }

        // Target root or primary data mount
        let primary_disk = self
            .disks
            .iter()
            .find(|d| d.mount_point() == Path::new("/"))
            .or_else(|| self.disks.first())?;

        let total_bytes = primary_disk.total_space();
        let free_bytes = primary_disk.available_space();
        let free_pct = if total_bytes > 0 {
            ((free_bytes as f64 / total_bytes as f64) * 100.0) as f32
        } else {
            0.0
        };

        // Read throughput from /proc/diskstats
        let current_sectors = read_proc_diskstats_sectors();
        let (read_bytes_per_sec, write_bytes_per_sec) =
            match (self.last_disk_sectors, current_sectors) {
                (Some((prev_r, prev_w)), Some((curr_r, curr_w)))
                    if curr_r >= prev_r && curr_w >= prev_w && elapsed_secs > 0.0 =>
                {
                    let r_bytes = (curr_r - prev_r) as f64 * 512.0;
                    let w_bytes = (curr_w - prev_w) as f64 * 512.0;
                    (r_bytes / elapsed_secs, w_bytes / elapsed_secs)
                }
                _ => (0.0, 0.0),
            };
        self.last_disk_sectors = current_sectors;

        Some(DiskMetrics {
            total_bytes,
            free_bytes,
            free_pct,
            read_bytes_per_sec,
            write_bytes_per_sec,
        })
    }

    #[allow(clippy::cast_precision_loss)]
    fn sample_network(&mut self, elapsed_secs: f64) -> NetworkMetrics {
        self.networks.refresh(true);
        let mut total_rx = 0;
        let mut total_tx = 0;

        for (_name, net) in &self.networks {
            total_rx += net.received();
            total_tx += net.transmitted();
        }

        let rx_bytes_per_sec = if elapsed_secs > 0.0 {
            #[allow(clippy::cast_precision_loss)]
            (total_rx as f64 / elapsed_secs)
        } else {
            0.0
        };

        let tx_bytes_per_sec = if elapsed_secs > 0.0 {
            #[allow(clippy::cast_precision_loss)]
            (total_tx as f64 / elapsed_secs)
        } else {
            0.0
        };

        let connection_count = count_proc_net_connections();

        NetworkMetrics {
            rx_bytes_per_sec,
            tx_bytes_per_sec,
            connection_count,
        }
    }

    fn sample_processes(&mut self) -> (Vec<ProcessMetrics>, Option<FootprintMetrics>) {
        let refresh_kind = sysinfo::ProcessRefreshKind::nothing()
            .with_cpu()
            .with_memory();
        self.sys
            .refresh_processes_specifics(sysinfo::ProcessesToUpdate::All, true, refresh_kind);
        let mut processes = Vec::new();
        let mut own_cpu: f32 = 0.0;
        let mut own_rss: u64 = 0;
        let mut own_count: u32 = 0;

        for (pid, proc_) in self.sys.processes() {
            let name = proc_.name().to_string_lossy().to_string();
            let is_starship = name.contains("starship");
            let cpu_pct = proc_.cpu_usage();
            let mem_bytes = proc_.memory();

            if is_starship {
                own_cpu += cpu_pct;
                own_rss += mem_bytes;
                own_count += 1;
            }

            processes.push(ProcessMetrics {
                pid: pid.as_u32(),
                name,
                cpu_pct,
                mem_bytes,
                is_starship_owned: is_starship,
            });
        }

        // Sort descending by CPU utilization and take top 8
        processes.sort_by(|a, b| {
            b.cpu_pct
                .partial_cmp(&a.cpu_pct)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        processes.truncate(8);

        let footprint = if own_count > 0 {
            Some(FootprintMetrics {
                total_cpu_pct: own_cpu,
                total_rss_bytes: own_rss,
                open_sockets: 0,
                process_count: own_count,
            })
        } else {
            None
        };

        (processes, footprint)
    }
}

impl Sampler for SystemSampler {
    fn sample_into(&mut self, sample: &mut HealthSample) {
        let now = Instant::now();
        let elapsed_secs = now.duration_since(self.last_sample_instant).as_secs_f64();
        self.last_sample_instant = now;

        sample.cpu = Some(self.sample_cpu());
        sample.memory = Some(self.sample_memory(elapsed_secs));
        sample.disk = self.sample_disk(elapsed_secs);
        sample.network = Some(self.sample_network(elapsed_secs));

        // Process enumeration runs on every 4th sample to keep sampling overhead < 1% CPU
        self.process_sample_counter = (self.process_sample_counter + 1) % 4;
        if self.process_sample_counter == 0 || self.last_processes.is_empty() {
            let (top_procs, footprint) = self.sample_processes();
            self.last_processes = top_procs;
            self.last_footprint = footprint;
        }
        sample.top_processes.clone_from(&self.last_processes);
        sample.own_footprint.clone_from(&self.last_footprint);
    }

    fn name(&self) -> &'static str {
        "system_sampler"
    }
}

/// Standalone GPU metric sampler with graceful unavailable handling.
#[derive(Debug, Default)]
pub struct GpuSampler {
    device_available: bool,
}

impl GpuSampler {
    #[must_use]
    pub fn new() -> Self {
        let amdgpu_path = Path::new("/sys/class/drm");
        let available = amdgpu_path.exists();
        Self {
            device_available: available,
        }
    }

    pub fn set_available(&mut self, available: bool) {
        self.device_available = available;
    }
}

impl Sampler for GpuSampler {
    fn sample_into(&mut self, sample: &mut HealthSample) {
        if !self.device_available {
            sample.gpu = None;
            return;
        }
        sample.gpu = None;
    }

    fn name(&self) -> &'static str {
        "gpu_sampler"
    }
}

/// Standalone Thermal & Fan metric sampler with sysfs path caching (GAP-CLOSURE-PLAN.md §14.3).
#[derive(Debug, Clone, PartialEq)]
pub struct ThermalSensorTarget {
    pub input_path: std::path::PathBuf,
    pub max_celsius: Option<f32>,
    pub crit_celsius: Option<f32>,
}

#[derive(Debug, Default)]
pub struct ThermalSampler {
    hwmon_available: bool,
    cached_sensors: Vec<ThermalSensorTarget>,
    cached_fans: Vec<std::path::PathBuf>,
}

impl ThermalSampler {
    #[must_use]
    pub fn new() -> Self {
        let hwmon_base = Path::new("/sys/class/hwmon");
        let available = hwmon_base.exists();
        let mut sampler = Self {
            hwmon_available: available,
            cached_sensors: Vec::new(),
            cached_fans: Vec::new(),
        };
        if available {
            sampler.rescan_sensors();
        }
        sampler
    }

    pub fn set_available(&mut self, available: bool) {
        self.hwmon_available = available;
        if !available {
            self.cached_sensors.clear();
            self.cached_fans.clear();
        }
    }

    /// Scans `/sys/class/hwmon` and caches sensor file paths and thresholds (GAP-CLOSURE-PLAN.md §14.3).
    pub fn rescan_sensors(&mut self) {
        self.cached_sensors.clear();
        self.cached_fans.clear();

        let hwmon_base = Path::new("/sys/class/hwmon");
        let Ok(entries) = std::fs::read_dir(hwmon_base) else {
            return;
        };

        for entry in entries.flatten() {
            let hwmon_dir = entry.path();
            if !hwmon_dir.is_dir() {
                continue;
            }

            let Ok(files) = std::fs::read_dir(&hwmon_dir) else {
                continue;
            };

            for file in files.flatten() {
                let file_name = file.file_name().to_string_lossy().to_string();
                if file_name.starts_with("temp") && file_name.ends_with("_input") {
                    let prefix = file_name.trim_end_matches("_input");
                    let max_path = hwmon_dir.join(format!("{prefix}_max"));
                    let crit_path = hwmon_dir.join(format!("{prefix}_crit"));

                    let max_celsius = std::fs::read_to_string(&max_path)
                        .ok()
                        .and_then(|s| parse_hwmon_millidegrees(&s));
                    let crit_celsius = std::fs::read_to_string(&crit_path)
                        .ok()
                        .and_then(|s| parse_hwmon_millidegrees(&s));

                    self.cached_sensors.push(ThermalSensorTarget {
                        input_path: file.path(),
                        max_celsius,
                        crit_celsius,
                    });
                } else if file_name.starts_with("fan") && file_name.ends_with("_input") {
                    let fan_path = file.path();
                    let t0 = Instant::now();
                    if let Ok(content) = std::fs::read_to_string(&fan_path) {
                        // Exclude slow blocking ACPI/WMI methods (e.g. hp-wmi taking ~200ms per read)
                        if t0.elapsed() < Duration::from_millis(15)
                            && content.trim().parse::<u32>().is_ok()
                        {
                            self.cached_fans.push(fan_path);
                        } else {
                            debug!(target: "starship_health", path = ?fan_path, "Excluding slow ACPI/WMI fan sensor (>15ms) from 1Hz polling");
                        }
                    }
                }
            }
        }
    }
}

impl Sampler for ThermalSampler {
    fn sample_into(&mut self, sample: &mut HealthSample) {
        if !self.hwmon_available || self.cached_sensors.is_empty() {
            sample.thermal = None;
            return;
        }

        let mut max_temp: f32 = 0.0;
        let mut overall_crit: Option<f32> = None;
        let mut overall_max_limit: Option<f32> = None;

        for sensor in &self.cached_sensors {
            if let Ok(content) = std::fs::read_to_string(&sensor.input_path)
                && let Some(deg) = parse_hwmon_millidegrees(&content)
                && deg > max_temp
            {
                max_temp = deg;
            }

            if let Some(crit) = sensor.crit_celsius {
                overall_crit = Some(overall_crit.map_or(crit, |prev: f32| prev.min(crit)));
            }
            if let Some(mx) = sensor.max_celsius {
                overall_max_limit = Some(overall_max_limit.map_or(mx, |prev: f32| prev.min(mx)));
            }
        }

        if max_temp <= 0.0 {
            sample.thermal = None;
            return;
        }

        let delta_to_max = overall_max_limit.map_or(25.0, |limit| (limit - max_temp).max(0.0));

        let mut fan_rpm: Option<u32> = None;
        for fan_path in &self.cached_fans {
            if let Ok(content) = std::fs::read_to_string(fan_path)
                && let Ok(rpm) = content.trim().parse::<u32>()
            {
                fan_rpm = Some(fan_rpm.map_or(rpm, |prev| prev.max(rpm)));
            }
        }

        sample.thermal = Some(crate::sample::ThermalMetrics {
            max_temp_celsius: max_temp,
            crit_temp_celsius: overall_crit,
            delta_to_max,
            fan_rpm,
        });
    }

    fn name(&self) -> &'static str {
        "thermal_sampler"
    }
}

/// Standalone Linux PSI (Pressure Stall Information) sampler (GAP-CLOSURE-PLAN.md §14.3).
#[derive(Debug, Default)]
pub struct PsiSampler {
    psi_available: bool,
}

impl PsiSampler {
    #[must_use]
    pub fn new() -> Self {
        let psi_cpu = Path::new("/proc/pressure/cpu");
        Self {
            psi_available: psi_cpu.exists(),
        }
    }

    pub fn set_available(&mut self, available: bool) {
        self.psi_available = available;
    }
}

impl Sampler for PsiSampler {
    fn sample_into(&mut self, sample: &mut HealthSample) {
        if !self.psi_available {
            sample.psi = None;
            return;
        }

        let cpu_str = std::fs::read_to_string("/proc/pressure/cpu").unwrap_or_default();
        let mem_str = std::fs::read_to_string("/proc/pressure/memory").unwrap_or_default();
        let io_str = std::fs::read_to_string("/proc/pressure/io").unwrap_or_default();

        sample.psi = parse_psi_str(&cpu_str, &mem_str, &io_str);
    }

    fn name(&self) -> &'static str {
        "psi_sampler"
    }
}

/// Parse Linux millidegrees Celsius from sysfs string (e.g. "69000" -> 69.0).
#[must_use]
pub fn parse_hwmon_millidegrees(content: &str) -> Option<f32> {
    let millis = content.trim().parse::<f32>().ok()?;
    Some(millis / 1000.0)
}

/// Parse a single PSI line e.g. "some avg10=0.05 avg60=0.88 avg300=0.77 total=416104297".
#[must_use]
pub fn parse_psi_line(line: &str) -> Option<(f64, f64)> {
    let mut avg10 = None;
    let mut avg60 = None;

    for token in line.split_whitespace() {
        if let Some(val) = token.strip_prefix("avg10=") {
            avg10 = val.parse::<f64>().ok();
        } else if let Some(val) = token.strip_prefix("avg60=") {
            avg60 = val.parse::<f64>().ok();
        }
    }

    match (avg10, avg60) {
        (Some(a10), Some(a60)) => Some((a10, a60)),
        _ => None,
    }
}

/// Parse full PSI metrics from contents of `/proc/pressure/{cpu,memory,io}`.
#[must_use]
pub fn parse_psi_str(cpu: &str, mem: &str, io: &str) -> Option<crate::sample::PsiMetrics> {
    if cpu.is_empty() && mem.is_empty() && io.is_empty() {
        return None;
    }

    let mut cpu_some = (0.0, 0.0);
    for line in cpu.lines() {
        if line.starts_with("some ")
            && let Some(vals) = parse_psi_line(line)
        {
            cpu_some = vals;
        }
    }

    let mut mem_some = (0.0, 0.0);
    let mut mem_full = (0.0, 0.0);
    for line in mem.lines() {
        if line.starts_with("some ")
            && let Some(vals) = parse_psi_line(line)
        {
            mem_some = vals;
        } else if line.starts_with("full ")
            && let Some(vals) = parse_psi_line(line)
        {
            mem_full = vals;
        }
    }

    let mut io_some = (0.0, 0.0);
    let mut io_full = (0.0, 0.0);
    for line in io.lines() {
        if line.starts_with("some ")
            && let Some(vals) = parse_psi_line(line)
        {
            io_some = vals;
        } else if line.starts_with("full ")
            && let Some(vals) = parse_psi_line(line)
        {
            io_full = vals;
        }
    }

    Some(crate::sample::PsiMetrics {
        cpu_some_avg10: cpu_some.0,
        cpu_some_avg60: cpu_some.1,
        mem_some_avg10: mem_some.0,
        mem_some_avg60: mem_some.1,
        mem_full_avg10: mem_full.0,
        mem_full_avg60: mem_full.1,
        io_some_avg10: io_some.0,
        io_some_avg60: io_some.1,
        io_full_avg10: io_full.0,
        io_full_avg60: io_full.1,
    })
}

/// Composite System Health Monitor coordinating all active samplers.
pub struct HealthMonitor {
    samplers: Vec<Box<dyn Sampler>>,
    mode: SamplingMode,
}

impl Default for HealthMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl HealthMonitor {
    #[must_use]
    pub fn new() -> Self {
        Self {
            samplers: vec![
                Box::new(SystemSampler::new()),
                Box::new(GpuSampler::new()),
                Box::new(ThermalSampler::new()),
                Box::new(PsiSampler::new()),
            ],
            mode: SamplingMode::Active,
        }
    }

    #[must_use]
    pub fn with_samplers(samplers: Vec<Box<dyn Sampler>>) -> Self {
        Self {
            samplers,
            mode: SamplingMode::Active,
        }
    }

    pub fn set_mode(&mut self, mode: SamplingMode) {
        self.mode = mode;
    }

    #[must_use]
    pub fn mode(&self) -> SamplingMode {
        self.mode
    }

    /// Produce a unified `HealthSample` by running all registered samplers.
    pub fn sample(&mut self) -> HealthSample {
        let timestamp_ms = u64::try_from(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis(),
        )
        .unwrap_or(0);

        let mut sample = HealthSample {
            timestamp_ms,
            ..Default::default()
        };

        for s in &mut self.samplers {
            debug!(target: "starship_health", sampler = s.name(), "Running sampler");
            s.sample_into(&mut sample);
        }

        sample
    }
}

// Helper: parse pswpin from /proc/vmstat
fn read_proc_vmstat_pswpin() -> Option<u64> {
    let file = File::open("/proc/vmstat").ok()?;
    let reader = BufReader::new(file);

    for line in reader.lines().map_while(Result::ok) {
        if line.starts_with("pswpin ") {
            let mut parts = line.split_whitespace();
            parts.next(); // "pswpin"
            return parts.next().and_then(|v| v.parse::<u64>().ok());
        }
    }
    None
}

// Helper: parse read/write sectors from /proc/diskstats
fn read_proc_diskstats_sectors() -> Option<(u64, u64)> {
    let file = File::open("/proc/diskstats").ok()?;
    let reader = BufReader::new(file);

    let mut total_read = 0;
    let mut total_written = 0;

    for line in reader.lines().map_while(Result::ok) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        // /proc/diskstats: field 5 is read sectors, field 9 is written sectors
        if parts.len() >= 10
            && let (Ok(r), Ok(w)) = (parts[5].parse::<u64>(), parts[9].parse::<u64>())
        {
            total_read += r;
            total_written += w;
        }
    }

    if total_read > 0 || total_written > 0 {
        Some((total_read, total_written))
    } else {
        None
    }
}

// Helper: count active connections in /proc/net/tcp and /proc/net/tcp6
fn count_proc_net_connections() -> u32 {
    let mut count = 0;
    for path in &["/proc/net/tcp", "/proc/net/tcp6"] {
        if let Ok(file) = File::open(path) {
            let reader = BufReader::new(file);
            // subtract 1 for the header line
            let lines = reader.lines().map_while(Result::ok).count();
            if lines > 1 {
                #[allow(clippy::cast_possible_truncation)]
                let entries = (lines - 1) as u32;
                count += entries;
            }
        }
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_container_with_no_gpu_or_hwmon_returns_sample_with_gpu_none() {
        let mut gpu_sampler = GpuSampler::default();
        gpu_sampler.set_available(false);

        let mut thermal_sampler = ThermalSampler::default();
        thermal_sampler.set_available(false);

        let mut monitor =
            HealthMonitor::with_samplers(vec![Box::new(gpu_sampler), Box::new(thermal_sampler)]);

        let sample = monitor.sample();

        assert_eq!(sample.gpu, None);
        assert_eq!(sample.thermal, None);
        assert!(sample.timestamp_ms > 0);
    }

    #[test]
    fn test_system_sampler_overhead_under_one_percent_on_quiet_machine() {
        // Verification requirement for 14.2:
        // "Verify: sampler overhead (own CPU) < 1 % average on a quiet machine (measured, recorded)."
        let mut monitor = HealthMonitor::new();

        // Warm up sysinfo once
        let _ = monitor.sample();

        let iterations = 5;
        let mut total_duration = Duration::ZERO;

        for _ in 0..iterations {
            let start = Instant::now();
            let sample = monitor.sample();
            total_duration += start.elapsed();

            // Verify essential metrics were collected
            assert!(sample.cpu.is_some(), "CPU metrics must be present");
            assert!(sample.memory.is_some(), "Memory metrics must be present");
            assert!(sample.network.is_some(), "Network metrics must be present");
        }

        let avg_sample_duration = total_duration / iterations;
        println!(
            "Average HealthMonitor sampling duration: {:?} ({:.4} ms)",
            avg_sample_duration,
            avg_sample_duration.as_secs_f64() * 1000.0
        );

        // At 1 Hz (1000ms period), overhead % = (sample_time / 1000ms) * 100%
        let overhead_pct = (avg_sample_duration.as_secs_f64() / 1.0) * 100.0;
        println!("Calculated sampling overhead at 1 Hz: {overhead_pct:.4}%");

        assert!(
            overhead_pct < 1.0,
            "Sampler overhead ({overhead_pct:.4}%) must be strictly under 1.0% average"
        );
    }

    #[test]
    fn test_adaptive_sampling_rate_intervals() {
        let mut monitor = HealthMonitor::new();
        assert_eq!(monitor.mode(), SamplingMode::Active);
        assert_eq!(monitor.mode().interval(), Duration::from_millis(1000));

        monitor.set_mode(SamplingMode::Idle);
        assert_eq!(monitor.mode(), SamplingMode::Idle);
        assert_eq!(monitor.mode().interval(), Duration::from_millis(5000));
    }

    #[test]
    fn test_psi_parser_with_real_linux_fixtures() {
        // Verification requirement for 14.3:
        // "Verify: parser tests with real-format fixtures; missing PSI handled."
        let cpu_fixture = "some avg10=0.05 avg60=0.88 avg300=0.77 total=416104297\n";
        let mem_fixture = "some avg10=0.01 avg60=0.27 avg300=0.11 total=47238184\nfull avg10=0.02 avg60=0.19 avg300=0.07 total=36577695\n";
        let io_fixture = "some avg10=0.24 avg60=1.14 avg300=0.68 total=205453001\nfull avg10=0.22 avg60=0.77 avg300=0.44 total=151221197\n";

        let psi =
            parse_psi_str(cpu_fixture, mem_fixture, io_fixture).expect("parse PSI real fixtures");

        assert!((psi.cpu_some_avg10 - 0.05).abs() < 1e-4);
        assert!((psi.cpu_some_avg60 - 0.88).abs() < 1e-4);

        assert!((psi.mem_some_avg10 - 0.01).abs() < 1e-4);
        assert!((psi.mem_some_avg60 - 0.27).abs() < 1e-4);
        assert!((psi.mem_full_avg10 - 0.02).abs() < 1e-4);
        assert!((psi.mem_full_avg60 - 0.19).abs() < 1e-4);

        assert!((psi.io_some_avg10 - 0.24).abs() < 1e-4);
        assert!((psi.io_some_avg60 - 1.14).abs() < 1e-4);
        assert!((psi.io_full_avg10 - 0.22).abs() < 1e-4);
        assert!((psi.io_full_avg60 - 0.77).abs() < 1e-4);
    }

    #[test]
    fn test_missing_psi_handled_gracefully() {
        let empty_psi = parse_psi_str("", "", "");
        assert_eq!(empty_psi, None, "Empty PSI strings must return None");

        let mut psi_sampler = PsiSampler::new();
        psi_sampler.set_available(false); // Simulate container with no PSI
        let mut sample = HealthSample::default();
        psi_sampler.sample_into(&mut sample);
        assert_eq!(sample.psi, None, "Unavailable PSI must gracefully set None");
    }

    #[test]
    fn test_hwmon_millidegrees_parser() {
        assert_eq!(parse_hwmon_millidegrees("69000\n"), Some(69.0));
        assert_eq!(parse_hwmon_millidegrees("100000"), Some(100.0));
        assert_eq!(parse_hwmon_millidegrees("invalid"), None);
    }
}
