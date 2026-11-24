use crate::runner::RunOutput;
use std::path::Path;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct BenchmarkMetrics {
    pub iterations: usize,
    pub mean_time_us: f64,
    pub min_time_us: u64,
    pub p95_time_us: u64,
    pub stddev_time_us: f64,
    pub binary_size_bytes: u64,
    pub compile_time_ms: u64,
    pub is_pareto_optimal: bool,
}

pub struct BenchmarkEvaluator;

impl BenchmarkEvaluator {
    /// Executes multiple iterations of the binary to record statistical metrics (ACE-010, ACE-012)
    pub fn evaluate_binary<F>(
        binary_path: &Path,
        iterations: usize,
        binary_size_bytes: u64,
        compile_duration: Duration,
        mut run_fn: F,
    ) -> Result<BenchmarkMetrics, String>
    where
        F: FnMut(&Path) -> Result<RunOutput, String>,
    {
        if iterations == 0 {
            return Err("Benchmark iterations must be at least 1".to_string());
        }

        let mut sample_durations_us = Vec::with_capacity(iterations);

        for _ in 0..iterations {
            let output = run_fn(binary_path)?;
            if output.timed_out {
                return Err("Benchmark run timed out (candidate exceeded time quota)".to_string());
            }
            let us = output.duration.as_micros() as u64;
            sample_durations_us.push(us);
        }

        sample_durations_us.sort_unstable();

        let sum: u64 = sample_durations_us.iter().sum();
        let mean = sum as f64 / iterations as f64;
        let min = *sample_durations_us.first().unwrap_or(&0);

        // 95th percentile
        let p95_idx = ((iterations as f64 * 0.95).ceil() as usize)
            .saturating_sub(1)
            .min(iterations - 1);
        let p95 = sample_durations_us[p95_idx];

        // Standard deviation (dispersion)
        let variance = sample_durations_us
            .iter()
            .map(|&v| {
                let diff = v as f64 - mean;
                diff * diff
            })
            .sum::<f64>()
            / iterations as f64;
        let stddev = variance.sqrt();

        Ok(BenchmarkMetrics {
            iterations,
            mean_time_us: mean,
            min_time_us: min,
            p95_time_us: p95,
            stddev_time_us: stddev,
            binary_size_bytes,
            compile_time_ms: compile_duration.as_millis() as u64,
            is_pareto_optimal: false, // Calculated across candidate set
        })
    }

    /// Identifies the Pareto-optimal frontier across multiple candidates (ACE-014)
    /// Objectives to minimize: mean_time_us, binary_size_bytes, compile_time_ms
    pub fn compute_pareto_frontier(candidates: &mut [(String, BenchmarkMetrics)]) {
        let n = candidates.len();
        if n == 0 {
            return;
        }

        let mut dominated = vec![false; n];

        for i in 0..n {
            for j in 0..n {
                if i == j {
                    continue;
                }
                let a = &candidates[j].1; // potential dominator
                let b = &candidates[i].1; // potential dominated

                // A dominates B if A is <= B on all metrics, and strictly < on at least one
                let time_better_or_equal = a.mean_time_us <= b.mean_time_us;
                let size_better_or_equal = a.binary_size_bytes <= b.binary_size_bytes;
                let compile_better_or_equal = a.compile_time_ms <= b.compile_time_ms;

                let strictly_better = a.mean_time_us < b.mean_time_us
                    || a.binary_size_bytes < b.binary_size_bytes
                    || a.compile_time_ms < b.compile_time_ms;

                if time_better_or_equal
                    && size_better_or_equal
                    && compile_better_or_equal
                    && strictly_better
                {
                    dominated[i] = true;
                    break;
                }
            }
        }

        for (i, (_, metrics)) in candidates.iter_mut().enumerate() {
            metrics.is_pareto_optimal = !dominated[i];
        }
    }
}
