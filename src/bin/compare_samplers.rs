// Copyright (c) Anza Technology, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Simple script to compare different sampling strategies.
//!
//! Compares both distribution quality (variance, fairness) and performance
//! (throughput, latency). Run with: cargo run --release --bin=compare_samplers
//!
//! # Performance metrics
//!
//! - **Single-sample**: Mean nanoseconds per `sample()` call and samples/sec.
//!   Simulates the cost of choosing one relay per shred (Rotor calls this once per shred).
//! - **Batch**: Mean ns per sample and total µs per `sample_multiple(64)` call, and samples/sec.
//!   Simulates the cost of choosing relays for a full slice of 64 shreds.
//!   Lower batch µs = less CPU per block of shreds.

use alpenglow::disseminator::rotor::sampling_strategy::{
    DecayingAcceptanceSampler, FaitAccompli1Sampler, FaitAccompli2Sampler, SamplingStrategy,
    StakeWeightedSampler, TurbineSampler, UniformSampler,
};
use alpenglow::shredder::TOTAL_SHREDS;
use alpenglow::network::dontcare_sockaddr;
use alpenglow::crypto::aggsig;
use alpenglow::crypto::signature::SecretKey;
use alpenglow::{ValidatorId, ValidatorInfo};
// use rand::rng;
use std::collections::HashMap;
use std::time::Instant;

/// Number of samples for single-sample throughput measurement.
const PERF_SINGLE_ITERATIONS: usize = 100_000;
/// Number of batches for batch throughput (each batch has TOTAL_SHREDS samples).
const PERF_BATCH_ITERATIONS: usize = 1_000;

fn create_test_validators(count: usize) -> Vec<ValidatorInfo> {
    let mut validators = Vec::new();
    for i in 0..count {
        let sk = SecretKey::new(&mut rand::rng());
        let voting_sk = aggsig::SecretKey::new(&mut rand::rng());
        
        // Create some stake distribution: first validator has 50%, rest share the other 50%
        let stake = if i == 0 {
            1_000_000_000
        } else {
            1_000_000_000 / (count - 1) as u64
        };
        
        validators.push(ValidatorInfo {
            id: i as ValidatorId,
            stake,
            pubkey: sk.to_pk(),
            voting_pubkey: voting_sk.to_pk(),
            all2all_address: dontcare_sockaddr(),
            disseminator_address: dontcare_sockaddr(),
            repair_request_address: dontcare_sockaddr(),
            repair_response_address: dontcare_sockaddr(),
        });
    }
    validators
}

fn analyze_sampler<S: SamplingStrategy + Clone>(
    name: &str,
    sampler: &S,
    num_samples: usize,
) {
    let mut rng = rand::rng();
    let mut counts: HashMap<ValidatorId, usize> = HashMap::new();
    
    // Sample many times
    // For strategies with state (like DecayingAcceptance), we need to clone
    let sampler = sampler.clone();
    for _ in 0..num_samples {
        let id = sampler.sample(&mut rng);
        *counts.entry(id).or_insert(0) += 1;
    }
    
    analyze_counts(name, &counts, num_samples);
}

fn analyze_counts(name: &str, counts: &HashMap<ValidatorId, usize>, num_samples: usize) {
    // Calculate statistics
    let mut sorted_counts: Vec<_> = counts.iter().collect();
    sorted_counts.sort_by_key(|&(_, count)| std::cmp::Reverse(*count));
    
    println!("\n=== {name} ===");
    println!("Total samples: {num_samples}");
    println!("Unique validators sampled: {}", counts.len());
    println!("\nTop 10 most sampled validators:");
    for (i, (validator_id, count)) in sorted_counts.iter().take(10).enumerate() {
        let percentage = (**count as f64 / num_samples as f64) * 100.0;
        println!("  {}. Validator {}: {} times ({:.2}%)", 
                 i + 1, validator_id, count, percentage);
    }
    
    // Calculate variance
    let mean = num_samples as f64 / counts.len() as f64;
    let variance: f64 = counts.values()
        .map(|&count| {
            let diff = count as f64 - mean;
            diff * diff
        })
        .sum::<f64>() / counts.len() as f64;
    let std_dev = variance.sqrt();
    println!("\nStatistics:");
    println!("  Mean samples per validator: {:.2}", mean);
    println!("  Standard deviation: {:.2}", std_dev);
    println!("  Coefficient of variation: {:.2}%", (std_dev / mean) * 100.0);
}

/// Measures single-sample performance: mean ns per sample and samples per second.
fn measure_single_sample_perf<S: SamplingStrategy + Clone>(
    sampler: &S,
    iterations: usize,
) -> (f64, f64) {
    let mut rng = rand::rng();
    let mut sampler = sampler.clone();
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = sampler.sample(&mut rng);
    }
    let elapsed = start.elapsed();
    let total_ns = elapsed.as_nanos() as f64;
    let mean_ns = total_ns / iterations as f64;
    let samples_per_sec = 1_000_000_000.0 * (iterations as f64) / total_ns;
    (mean_ns, samples_per_sec)
}

/// Measures single-sample performance for DecayingAcceptanceSampler, which needs
/// periodic reset (every `reset_every` samples) to avoid rejecting all samples.
fn measure_single_sample_perf_decaying(
    sampler: &DecayingAcceptanceSampler,
    iterations: usize,
    reset_every: usize,
) -> (f64, f64) {
    let mut rng = rand::rng();
    let sampler = sampler.clone();
    let start = Instant::now();
    for i in 0..iterations {
        if i > 0 && i % reset_every == 0 {
            sampler.reset();
        }
        let _ = sampler.sample(&mut rng);
    }
    let elapsed = start.elapsed();
    let total_ns = elapsed.as_nanos() as f64;
    let mean_ns = total_ns / iterations as f64;
    let samples_per_sec = 1_000_000_000.0 * (iterations as f64) / total_ns;
    (mean_ns, samples_per_sec)
}

/// Measures batch sampling performance (sample_multiple). Returns (mean_ns_per_sample, mean_batch_ns, samples_per_sec).
fn measure_batch_perf<S: SamplingStrategy + Clone>(
    sampler: &S,
    batch_size: usize,
    num_batches: usize,
) -> (f64, f64, f64) {
    let mut rng = rand::rng();
    let start = Instant::now();
    for _ in 0..num_batches {
        let _ = sampler.sample_multiple(batch_size, &mut rng);
    }
    let elapsed = start.elapsed();
    let total_ns = elapsed.as_nanos() as f64;
    let total_samples = num_batches * batch_size;
    let mean_ns_per_sample = total_ns / total_samples as f64;
    let mean_batch_ns = total_ns / num_batches as f64;
    let samples_per_sec = 1_000_000_000.0 * (total_samples as f64) / total_ns;
    (mean_ns_per_sample, mean_batch_ns, samples_per_sec)
}

/// Batch performance for DecayingAcceptanceSampler (resets before each batch).
fn measure_batch_perf_decaying(
    sampler: &DecayingAcceptanceSampler,
    batch_size: usize,
    num_batches: usize,
) -> (f64, f64, f64) {
    let mut rng = rand::rng();
    let sampler = sampler.clone();
    let start = Instant::now();
    for _ in 0..num_batches {
        sampler.reset();
        let _ = sampler.sample_multiple(batch_size, &mut rng);
    }
    let elapsed = start.elapsed();
    let total_ns = elapsed.as_nanos() as f64;
    let total_samples = num_batches * batch_size;
    let mean_ns_per_sample = total_ns / total_samples as f64;
    let mean_batch_ns = total_ns / num_batches as f64;
    let samples_per_sec = 1_000_000_000.0 * (total_samples as f64) / total_ns;
    (mean_ns_per_sample, mean_batch_ns, samples_per_sec)
}

fn main() {
    println!("Comparing Sampling Strategies");
    println!("=============================\n");
    
    let validators = create_test_validators(100);
    let num_samples = 10_000;
    
    // Test each strategy
    println!("Test setup:");
    println!("  Validators: {}", validators.len());
    println!("  Total samples per strategy: {num_samples}");
    println!("  Stake distribution: Validator 0 has 50%, others share 50%");
    
    // Uniform
    let uniform = UniformSampler::new(validators.clone());
    analyze_sampler("UniformSampler", &uniform, num_samples);
    
    // Stake Weighted
    let stake_weighted = StakeWeightedSampler::new(validators.clone());
    analyze_sampler("StakeWeightedSampler", &stake_weighted, num_samples);
    
    // FA1 with IID fallback - use sample_multiple for proper behavior
    {
        let fa1_iid = FaitAccompli1Sampler::new_with_stake_weighted_fallback(
            validators.clone(),
            TOTAL_SHREDS as u64,
        );
        let mut rng = rand::rng();
        let mut counts: HashMap<ValidatorId, usize> = HashMap::new();
        
        // FA1 is designed for sample_multiple, so sample in batches
        for _ in 0..(num_samples / TOTAL_SHREDS) {
            let samples = fa1_iid.sample_multiple(TOTAL_SHREDS, &mut rng);
            for id in samples {
                *counts.entry(id).or_insert(0) += 1;
            }
        }
        
        analyze_counts("FaitAccompli1Sampler (IID)", &counts, num_samples);
    }
    
    // FA1 with Partition fallback
    {
        let fa1_partition = FaitAccompli1Sampler::new_with_partition_fallback(
            validators.clone(),
            TOTAL_SHREDS as u64,
        );
        let mut rng = rand::rng();
        let mut counts: HashMap<ValidatorId, usize> = HashMap::new();
        
        for _ in 0..(num_samples / TOTAL_SHREDS) {
            let samples = fa1_partition.sample_multiple(TOTAL_SHREDS, &mut rng);
            for id in samples {
                *counts.entry(id).or_insert(0) += 1;
            }
        }
        
        analyze_counts("FaitAccompli1Sampler (Partition)", &counts, num_samples);
    }
    
    // FA2
    {
        let fa2 = FaitAccompli2Sampler::new(validators.clone(), TOTAL_SHREDS as u64);
        let mut rng = rand::rng();
        let mut counts: HashMap<ValidatorId, usize> = HashMap::new();
        
        for _ in 0..(num_samples / TOTAL_SHREDS) {
            let samples = fa2.sample_multiple(TOTAL_SHREDS, &mut rng);
            for id in samples {
                *counts.entry(id).or_insert(0) += 1;
            }
        }
        
        analyze_counts("FaitAccompli2Sampler", &counts, num_samples);
    }
    
    // Decaying Acceptance - reset between samples for fair comparison
    let decaying = DecayingAcceptanceSampler::new(validators.clone(), 3.0);
    {
        let mut rng = rand::rng();
        let mut counts: HashMap<ValidatorId, usize> = HashMap::new();
        let sampler = decaying.clone();
        
        // Reset after each batch to simulate fresh sampling
        for _ in 0..(num_samples / 100) {
            sampler.reset();
            for _ in 0..100 {
                let id = sampler.sample(&mut rng);
                *counts.entry(id).or_insert(0) += 1;
            }
        }
        
        analyze_counts("DecayingAcceptanceSampler (max=3.0)", &counts, num_samples);
    }
    
    // Turbine
    let turbine = TurbineSampler::new(validators.clone());
    analyze_sampler("TurbineSampler", &turbine, num_samples);

    // -------------------------------------------------------------------------
    // Performance metrics
    // -------------------------------------------------------------------------
    println!("\n\n");
    println!("Performance Metrics");
    println!("===================");
    println!(
        "Single-sample: {} iterations, batch: {} batches of {} (ns = nanoseconds)",
        PERF_SINGLE_ITERATIONS, PERF_BATCH_ITERATIONS, TOTAL_SHREDS
    );
    println!();

    // Single-sample performance
    println!("Single-sample (one sample() call):");
    println!(
        "  {:<38} {:>14} {:>18}",
        "Strategy", "Mean (ns/sample)", "Samples/sec"
    );
    println!(
        "  {} {} {}",
        "-".repeat(38),
        "-".repeat(14),
        "-".repeat(18)
    );

    let uniform = UniformSampler::new(validators.clone());
    let (mean_ns, samples_per_sec) =
        measure_single_sample_perf(&uniform, PERF_SINGLE_ITERATIONS);
    println!("  {:<38} {:>14.0} {:>18.0}", "UniformSampler", mean_ns, samples_per_sec);

    let stake_weighted = StakeWeightedSampler::new(validators.clone());
    let (mean_ns, samples_per_sec) =
        measure_single_sample_perf(&stake_weighted, PERF_SINGLE_ITERATIONS);
    println!(
        "  {:<38} {:>14.0} {:>18.0}",
        "StakeWeightedSampler", mean_ns, samples_per_sec
    );

    let fa1_iid = FaitAccompli1Sampler::new_with_stake_weighted_fallback(
        validators.clone(),
        TOTAL_SHREDS as u64,
    );
    let (mean_ns, samples_per_sec) =
        measure_single_sample_perf(&fa1_iid, PERF_SINGLE_ITERATIONS);
    println!(
        "  {:<38} {:>14.0} {:>18.0}",
        "FaitAccompli1Sampler (IID)", mean_ns, samples_per_sec
    );

    let fa1_partition = FaitAccompli1Sampler::new_with_partition_fallback(
        validators.clone(),
        TOTAL_SHREDS as u64,
    );
    let (mean_ns, samples_per_sec) =
        measure_single_sample_perf(&fa1_partition, PERF_SINGLE_ITERATIONS);
    println!(
        "  {:<38} {:>14.0} {:>18.0}",
        "FaitAccompli1Sampler (Partition)", mean_ns, samples_per_sec
    );

    let fa2 = FaitAccompli2Sampler::new(validators.clone(), TOTAL_SHREDS as u64);
    let (mean_ns, samples_per_sec) =
        measure_single_sample_perf(&fa2, PERF_SINGLE_ITERATIONS);
    println!(
        "  {:<38} {:>14.0} {:>18.0}",
        "FaitAccompli2Sampler", mean_ns, samples_per_sec
    );

    let decaying = DecayingAcceptanceSampler::new(validators.clone(), 3.0);
    let (mean_ns, samples_per_sec) = measure_single_sample_perf_decaying(
        &decaying,
        PERF_SINGLE_ITERATIONS,
        64, // reset every 64 samples (one block of shreds)
    );
    println!(
        "  {:<38} {:>14.0} {:>18.0}",
        "DecayingAcceptanceSampler (max=3.0)", mean_ns, samples_per_sec
    );

    let turbine = TurbineSampler::new(validators.clone());
    let (mean_ns, samples_per_sec) =
        measure_single_sample_perf(&turbine, PERF_SINGLE_ITERATIONS);
    println!(
        "  {:<38} {:>14.0} {:>18.0}",
        "TurbineSampler", mean_ns, samples_per_sec
    );

    // Batch performance (sample_multiple(TOTAL_SHREDS))
    println!();
    println!(
        "Batch (sample_multiple({})) — cost per batch of shreds:",
        TOTAL_SHREDS
    );
    println!(
        "  {:<38} {:>14} {:>14} {:>18}",
        "Strategy", "Ns/sample", "Batch (µs)", "Samples/sec"
    );
    println!(
        "  {} {} {} {}",
        "-".repeat(38),
        "-".repeat(14),
        "-".repeat(14),
        "-".repeat(18)
    );

    let (a, b, c) = measure_batch_perf(&uniform, TOTAL_SHREDS, PERF_BATCH_ITERATIONS);
    println!(
        "  {:<38} {:>14.0} {:>14.2} {:>18.0}",
        "UniformSampler", a, b / 1000.0, c
    );
    let (a, b, c) = measure_batch_perf(&stake_weighted, TOTAL_SHREDS, PERF_BATCH_ITERATIONS);
    println!(
        "  {:<38} {:>14.0} {:>14.2} {:>18.0}",
        "StakeWeightedSampler", a, b / 1000.0, c
    );
    let (a, b, c) = measure_batch_perf(&fa1_iid, TOTAL_SHREDS, PERF_BATCH_ITERATIONS);
    println!(
        "  {:<38} {:>14.0} {:>14.2} {:>18.0}",
        "FaitAccompli1Sampler (IID)", a, b / 1000.0, c
    );
    let (a, b, c) = measure_batch_perf(&fa1_partition, TOTAL_SHREDS, PERF_BATCH_ITERATIONS);
    println!(
        "  {:<38} {:>14.0} {:>14.2} {:>18.0}",
        "FaitAccompli1Sampler (Partition)", a, b / 1000.0, c
    );
    let (a, b, c) = measure_batch_perf(&fa2, TOTAL_SHREDS, PERF_BATCH_ITERATIONS);
    println!(
        "  {:<38} {:>14.0} {:>14.2} {:>18.0}",
        "FaitAccompli2Sampler", a, b / 1000.0, c
    );
    let (a, b, c) =
        measure_batch_perf_decaying(&decaying, TOTAL_SHREDS, PERF_BATCH_ITERATIONS);
    println!(
        "  {:<38} {:>14.0} {:>14.2} {:>18.0}",
        "DecayingAcceptanceSampler (max=3.0)", a, b / 1000.0, c
    );
    let (a, b, c) = measure_batch_perf(&turbine, TOTAL_SHREDS, PERF_BATCH_ITERATIONS);
    println!(
        "  {:<38} {:>14.0} {:>14.2} {:>18.0}",
        "TurbineSampler", a, b / 1000.0, c
    );

    println!("\n\nComparison complete!");
}
