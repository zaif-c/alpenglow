// Copyright (c) Anza Technology, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Simple script to compare different sampling strategies.
//!
//! Run with: cargo run --release --bin=compare_samplers

use alpenglow::disseminator::rotor::sampling_strategy::{
    DecayingAcceptanceSampler, FaitAccompli1Sampler, FaitAccompli2Sampler, SamplingStrategy,
    StakeWeightedSampler, TurbineSampler, UniformSampler,
};
use alpenglow::shredder::TOTAL_SHREDS;
use alpenglow::network::dontcare_sockaddr;
use alpenglow::crypto::aggsig;
use alpenglow::crypto::signature::SecretKey;
use alpenglow::{ValidatorId, ValidatorInfo};
use rand::{RngCore, rng};
use std::collections::HashMap;

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
    
    println!("\n\nComparison complete!");
}
