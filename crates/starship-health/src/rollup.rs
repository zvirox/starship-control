use crate::sample::HealthSample;

/// Summary metrics aggregated over a 1-minute or 1-hour window.
#[derive(Debug, Clone, PartialEq)]
pub struct AggregatedHealth {
    pub cpu_avg: f64,
    pub cpu_max: f64,
    pub mem_avail_pct_min: f64,
    pub swap_in_max: f64,
    pub disk_free_pct_min: f64,
    pub temp_max: f64,
    pub gpu_util_max: Option<f64>,
    pub vram_pct_max: Option<f64>,
    pub psi_cpu_max: Option<f64>,
    pub psi_mem_max: Option<f64>,
    pub psi_io_max: Option<f64>,
}

/// Aggregates a slice of `HealthSample`s into an `AggregatedHealth` summary.
#[must_use]
pub fn aggregate_samples(samples: &[HealthSample]) -> Option<AggregatedHealth> {
    if samples.is_empty() {
        return None;
    }

    let mut cpu_sum = 0.0;
    let mut cpu_count = 0;
    let mut cpu_max: f64 = 0.0;

    let mut mem_avail_pct_min: f64 = 100.0;
    let mut swap_in_max: f64 = 0.0;
    let mut disk_free_pct_min: f64 = 100.0;
    let mut temp_max: f64 = 0.0;

    let mut gpu_util_max: Option<f64> = None;
    let mut vram_pct_max: Option<f64> = None;

    let mut psi_cpu_max: Option<f64> = None;
    let mut psi_mem_max: Option<f64> = None;
    let mut psi_io_max: Option<f64> = None;

    for s in samples {
        if let Some(cpu) = &s.cpu {
            let u = f64::from(cpu.global_util_pct);
            cpu_sum += u;
            cpu_count += 1;
            if u > cpu_max {
                cpu_max = u;
            }
        }

        if let Some(mem) = &s.memory {
            let av = f64::from(mem.available_pct);
            if av < mem_avail_pct_min {
                mem_avail_pct_min = av;
            }
            if mem.swap_in_bytes_per_sec > swap_in_max {
                swap_in_max = mem.swap_in_bytes_per_sec;
            }
        }

        if let Some(disk) = &s.disk {
            let free = f64::from(disk.free_pct);
            if free < disk_free_pct_min {
                disk_free_pct_min = free;
            }
        }

        if let Some(thermal) = &s.thermal {
            let t = f64::from(thermal.max_temp_celsius);
            if t > temp_max {
                temp_max = t;
            }
        }

        if let Some(gpu) = &s.gpu {
            let gu = f64::from(gpu.util_pct);
            gpu_util_max = Some(gpu_util_max.map_or(gu, |prev| prev.max(gu)));

            let vu = f64::from(gpu.vram_used_pct);
            vram_pct_max = Some(vram_pct_max.map_or(vu, |prev| prev.max(vu)));
        }

        if let Some(psi) = &s.psi {
            let pc = psi.cpu_some_avg10;
            psi_cpu_max = Some(psi_cpu_max.map_or(pc, |prev| prev.max(pc)));

            let pm = psi.mem_some_avg10;
            psi_mem_max = Some(psi_mem_max.map_or(pm, |prev| prev.max(pm)));

            let pi = psi.io_some_avg10;
            psi_io_max = Some(psi_io_max.map_or(pi, |prev| prev.max(pi)));
        }
    }

    let cpu_avg = if cpu_count > 0 {
        cpu_sum / f64::from(cpu_count)
    } else {
        0.0
    };

    Some(AggregatedHealth {
        cpu_avg,
        cpu_max,
        mem_avail_pct_min,
        swap_in_max,
        disk_free_pct_min,
        temp_max,
        gpu_util_max,
        vram_pct_max,
        psi_cpu_max,
        psi_mem_max,
        psi_io_max,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sample::{CpuMetrics, MemoryMetrics};

    #[test]
    fn test_aggregate_samples_computes_expected_averages_and_extrema() {
        let mut samples = Vec::new();
        for i in 1..=10 {
            #[allow(clippy::cast_precision_loss)]
            let util = (i * 5) as f32; // 5%, 10%, ..., 50%
            samples.push(HealthSample {
                timestamp_ms: i * 1000,
                cpu: Some(CpuMetrics {
                    global_util_pct: util,
                    core_util_pct: vec![util],
                    load_avg_1m: 1.0,
                    load_avg_5m: 1.0,
                    load_avg_15m: 1.0,
                    avg_freq_mhz: 3200,
                }),
                memory: Some(MemoryMetrics {
                    total_bytes: 16_000_000_000,
                    used_bytes: 8_000_000_000,
                    available_bytes: 8_000_000_000,
                    #[allow(clippy::cast_precision_loss)]
                    available_pct: (50.0 - (i as f32)),
                    swap_total_bytes: 0,
                    swap_used_bytes: 0,
                    swap_in_bytes_per_sec: 0.0,
                }),
                ..Default::default()
            });
        }

        let agg = aggregate_samples(&samples).expect("aggregate samples");
        // Average CPU = sum(5..=50) / 10 = 275 / 10 = 27.5
        assert!((agg.cpu_avg - 27.5).abs() < 1e-4);
        assert!((agg.cpu_max - 50.0).abs() < 1e-4);
        // Min available pct = 50 - 10 = 40.0%
        assert!((agg.mem_avail_pct_min - 40.0).abs() < 1e-4);
    }
}
