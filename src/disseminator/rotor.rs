// Copyright (c) Anza Technology, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Implementation of Alpenglow's new Rotor block dissemination protocol.
//!
//! This is an evolution of Solana's original Turbine block dissemination protocol.
//! Instead of a multi-layered tree, it always uses a single layer of relayers.
//!
//! Rotor can be instantiated with any quorum sampling strategy.
//! Therefore, this module also provides multiple implementation of such.
//! See also, the [`sampling_strategy`] module and the [`SamplingStrategy`] trait.
//!
//! For an implementation of Turbine, see [`crate::disseminator::turbine::Turbine`].

pub mod sampling_strategy;

use std::sync::Arc;

use async_trait::async_trait;
use rand::prelude::*;
use log::info;

use self::sampling_strategy::{
    AllSameSampler, DecayingAcceptanceSampler, PartitionSampler, TurbineSampler, UniformSampler,
};
pub use self::sampling_strategy::{
    FaitAccompli1Sampler, FaitAccompli2Sampler, SamplingStrategy, StakeWeightedSampler,
};
use super::Disseminator;
use crate::consensus::EpochInfo;
use crate::network::{Network, ShredNetwork};
use crate::shredder::{Shred, TOTAL_SHREDS};
use crate::{Slot, ValidatorId};

/// Rotor is a new block dissemination protocol presented together with Alpenglow.
pub struct Rotor<N: Network, S: SamplingStrategy> {
    network: N,
    sampler: S,
    epoch_info: Arc<EpochInfo>,
}

impl<N: Network> Rotor<N, StakeWeightedSampler> {
    /// Creates a new Rotor instance with the default sampling strategy.
    ///
    /// Contact information for all validators is provided in `validators`.
    /// Provided `network` will be used to send and receive shreds.
    pub fn new(network: N, epoch_info: Arc<EpochInfo>) -> Self {
        let validators = epoch_info.validators.clone();
        let sampler = StakeWeightedSampler::new(validators);
        Self {
            network,
            sampler,
            epoch_info,
        }
    }
}

impl<N: Network> Rotor<N, FaitAccompli1Sampler<PartitionSampler>> {
    /// Creates a new Rotor instance with the FA1 sampling strategy.
    ///
    /// Contact information for all validators is provided in `validators`.
    /// Provided `network` will be used to send and receive shreds.
    pub fn new_fa1(network: N, epoch_info: Arc<EpochInfo>) -> Self {
        let validators = epoch_info.validators.clone();
        let sampler =
            FaitAccompli1Sampler::new_with_partition_fallback(validators, TOTAL_SHREDS as u64);
        Self {
            network,
            sampler,
            epoch_info,
        }
    }
}

impl<N: Network> Rotor<N, UniformSampler> {
    /// Creates a new Rotor instance with uniform sampling strategy.
    pub fn new_uniform(network: N, epoch_info: Arc<EpochInfo>) -> Self {
        let validators = epoch_info.validators.clone();
        let sampler = UniformSampler::new(validators);
        Self {
            network,
            sampler,
            epoch_info,
        }
    }
}

impl<N: Network> Rotor<N, DecayingAcceptanceSampler> {
    /// Creates a new Rotor instance with the decaying acceptance sampling strategy.
    pub fn new_decaying(network: N, epoch_info: Arc<EpochInfo>, max_samples: f64) -> Self {
        let validators = epoch_info.validators.clone();
        let sampler = DecayingAcceptanceSampler::new(validators, max_samples);
        Self {
            network,
            sampler,
            epoch_info,
        }
    }
}

impl<N: Network> Rotor<N, TurbineSampler> {
    /// Creates a new Rotor instance with the Turbine-inspired sampling strategy.
    pub fn new_turbine(network: N, epoch_info: Arc<EpochInfo>) -> Self {
        let validators = epoch_info.validators.clone();
        let sampler = TurbineSampler::new(validators);
        Self {
            network,
            sampler,
            epoch_info,
        }
    }
}

impl<N: Network> Rotor<N, FaitAccompli2Sampler> {
    /// Creates a new Rotor instance with the FA2 sampling strategy.
    pub fn new_fa2(network: N, epoch_info: Arc<EpochInfo>) -> Self {
        let validators = epoch_info.validators.clone();
        let sampler = FaitAccompli2Sampler::new(validators, TOTAL_SHREDS as u64);
        Self {
            network,
            sampler,
            epoch_info,
        }
    }
}

impl<N, S: SamplingStrategy> Rotor<N, S>
where
    N: ShredNetwork,
{
    /// Turns this instance into a new instance with a different sampling strategy.
    #[must_use]
    pub fn with_sampler(self, sampler: S) -> Self {
        Self { sampler, ..self }
    }

    /// Sends the shred to the correct relay.
    async fn send_as_leader(&self, shred: &Shred) -> std::io::Result<()> {
        let relay = self.sample_relay(shred.payload().header.slot, shred.payload().index_in_slot());
        let v = self.epoch_info.validator(relay);
        // Benchmarking instrumentation:
        // Count leader-to-relay transmissions for Rotor throughput metrics.
        info!("METRIC_RELAY leader_shreds=1");
        self.network.send(shred, v.disseminator_address).await
    }

    /// Broadcasts a shred to all validators except for the leader and itself.
    /// Does nothing if we are not the dedicated relay for this shred.
    async fn broadcast_if_relay(&self, shred: &Shred) -> std::io::Result<()> {
        let leader = self.epoch_info.leader(shred.payload().header.slot).id;

        // do nothing if we are not the relay
        let relay = self.sample_relay(shred.payload().header.slot, shred.payload().index_in_slot());
        if self.epoch_info.own_id != relay {
            return Ok(());
        }

        // otherwise, broadcast
        // Benchmarking instrumentation:
        // Count how many peers we relay to for this shred (fanout).
        let fanout = self
            .epoch_info
            .validators
            .iter()
            .filter(|v| v.id != leader && v.id != relay)
            .count();
        info!("METRIC_RELAY relay_shreds={fanout}");

        let to = self
            .epoch_info
            .validators
            .iter()
            .filter(|v| v.id != leader && v.id != relay)
            .map(|v| v.disseminator_address);
        self.network.send_to_many(shred, to).await?;
        Ok(())
    }

    fn sample_relay(&self, slot: Slot, shred: usize) -> ValidatorId {
        let seed = [
            slot.inner().to_be_bytes(),
            shred.to_be_bytes(),
            [0; 8],
            [0; 8],
        ]
        .concat();
        let mut rng = StdRng::from_seed(seed.try_into().unwrap());
        self.sampler.sample(&mut rng)
    }
}

/// Helper trait to construct a [`Rotor`] for a given sampling strategy.
///
/// This allows higher-level code (like the `node` binary) to only choose the
/// sampler type via a type alias, and rely on the appropriate [`Rotor`]
/// constructor being called automatically.
pub trait RotorSamplerExt: SamplingStrategy + Sized {
    fn build_rotor<N: Network>(network: N, epoch_info: Arc<EpochInfo>) -> Rotor<N, Self>;
}

impl RotorSamplerExt for StakeWeightedSampler {
    fn build_rotor<N: Network>(network: N, epoch_info: Arc<EpochInfo>) -> Rotor<N, Self> {
        Rotor::new(network, epoch_info)
    }
}

impl RotorSamplerExt for FaitAccompli1Sampler<PartitionSampler> {
    fn build_rotor<N: Network>(network: N, epoch_info: Arc<EpochInfo>) -> Rotor<N, Self> {
        Rotor::new_fa1(network, epoch_info)
    }
}

impl RotorSamplerExt for UniformSampler {
    fn build_rotor<N: Network>(network: N, epoch_info: Arc<EpochInfo>) -> Rotor<N, Self> {
        Rotor::new_uniform(network, epoch_info)
    }
}

// This sampler is not currently compatible with Rotor, but is included for completeness.
impl RotorSamplerExt for DecayingAcceptanceSampler {
    fn build_rotor<N: Network>(network: N, epoch_info: Arc<EpochInfo>) -> Rotor<N, Self> {
        // Use a reasonable default for max_samples; adjust here if needed.
        Rotor::new_decaying(network, epoch_info, 3.0)
    }
}

impl RotorSamplerExt for TurbineSampler {
    fn build_rotor<N: Network>(network: N, epoch_info: Arc<EpochInfo>) -> Rotor<N, Self> {
        Rotor::new_turbine(network, epoch_info)
    }
}

impl RotorSamplerExt for FaitAccompli2Sampler {
    fn build_rotor<N: Network>(network: N, epoch_info: Arc<EpochInfo>) -> Rotor<N, Self> {
        Rotor::new_fa2(network, epoch_info)
    }
}

impl RotorSamplerExt for AllSameSampler {
    fn build_rotor<N: Network>(network: N, epoch_info: Arc<EpochInfo>) -> Rotor<N, Self> {
        let first = epoch_info
            .validators
            .first()
            .cloned()
            .expect("at least one validator required for AllSameSampler");
        let sampler = AllSameSampler(first);
        Rotor {
            network,
            sampler,
            epoch_info,
        }
    }
}

impl RotorSamplerExt for PartitionSampler {
    fn build_rotor<N: Network>(network: N, epoch_info: Arc<EpochInfo>) -> Rotor<N, Self> {
        let validators = epoch_info.validators.clone();
        let num_bins = validators.len().max(1);
        let sampler = PartitionSampler::new(validators, num_bins);
        Rotor {
            network,
            sampler,
            epoch_info,
        }
    }
}

impl RotorSamplerExt for FaitAccompli1Sampler<StakeWeightedSampler> {
    fn build_rotor<N: Network>(network: N, epoch_info: Arc<EpochInfo>) -> Rotor<N, Self> {
        let validators = epoch_info.validators.clone();
        let sampler =
            FaitAccompli1Sampler::new_with_stake_weighted_fallback(validators, TOTAL_SHREDS as u64);
        Rotor {
            network,
            sampler,
            epoch_info,
        }
    }
}

#[async_trait]
impl<N, S: SamplingStrategy + Send + Sync + 'static> Disseminator for Rotor<N, S>
where
    N: ShredNetwork,
{
    async fn send(&self, shred: &Shred) -> std::io::Result<()> {
        Self::send_as_leader(self, shred).await
    }

    async fn forward(&self, shred: &Shred) -> std::io::Result<()> {
        Self::broadcast_if_relay(self, shred).await
    }

    async fn receive(&self) -> std::io::Result<Shred> {
        self.network.receive().await
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::sync::Arc;
    use std::time::Duration;

    use tokio::sync::Mutex;
    use tokio::task;

    use super::*;
    use crate::ValidatorInfo;
    use crate::crypto::aggsig;
    use crate::crypto::signature::SecretKey;
    use crate::network::{UdpNetwork, dontcare_sockaddr, localhost_ip_sockaddr};
    use crate::shredder::{MAX_DATA_PER_SLICE, RegularShredder, Shredder, TOTAL_SHREDS};
    use crate::types::slice::create_slice_with_invalid_txs;

    type MyRotor = Rotor<UdpNetwork<Shred, Shred>, StakeWeightedSampler>;

    fn create_rotor_instances(count: u64, base_port: u16) -> (Vec<SecretKey>, Vec<MyRotor>) {
        let mut sks = Vec::new();
        let mut voting_sks = Vec::new();
        let mut validators = Vec::new();
        for i in 0..count {
            sks.push(SecretKey::new(&mut rand::rng()));
            voting_sks.push(aggsig::SecretKey::new(&mut rand::rng()));
            validators.push(ValidatorInfo {
                id: i,
                stake: 1,
                pubkey: sks[i as usize].to_pk(),
                voting_pubkey: voting_sks[i as usize].to_pk(),
                all2all_address: dontcare_sockaddr(),
                disseminator_address: localhost_ip_sockaddr(base_port + i as u16),
                repair_request_address: dontcare_sockaddr(),
                repair_response_address: dontcare_sockaddr(),
            });
        }

        let mut rotors = Vec::new();
        for i in 0..count {
            let epoch_info = Arc::new(EpochInfo::new(i, validators.clone()));
            let network = UdpNetwork::new(base_port + i as u16);
            rotors.push(Rotor::new(network, epoch_info));
        }
        (sks, rotors)
    }

    async fn test_rotor_dissemination(count: u64, base_port: u16) {
        let (sks, mut rotors) = create_rotor_instances(count, base_port);
        let slice = create_slice_with_invalid_txs(MAX_DATA_PER_SLICE);
        let shreds = RegularShredder::default().shred(slice, &sks[0]).unwrap();

        let mut shreds_received = Vec::with_capacity(rotors.len());
        (0..rotors.len()).for_each(|_| shreds_received.push(Arc::new(Mutex::new(HashSet::new()))));
        let mut rotor_tasks = Vec::with_capacity(rotors.len());

        // forward & receive shreds on "non-leader" Rotor instance
        for i in 0..rotors.len() - 1 {
            let shreds_received = shreds_received[i].clone();
            let rotor_non_leader = rotors.pop().unwrap();
            rotor_tasks.push(task::spawn(async move {
                loop {
                    match rotor_non_leader.receive().await {
                        Ok(shred) => {
                            rotor_non_leader.forward(&shred).await.unwrap();
                            let mut guard = shreds_received.lock().await;
                            assert!(!guard.contains(&*shred.payload().shred_index));
                            guard.insert(*shred.payload().shred_index);
                        }
                        _ => continue,
                    }
                }
            }));
        }

        tokio::time::sleep(Duration::from_millis(10)).await;

        assert_eq!(rotors.len(), 1);
        for shred in shreds {
            rotors[0].send(&shred).await.unwrap();
        }

        // forward shreds on the "leader" Rotor instance
        let rotor_leader = rotors.pop().unwrap();
        let rotor_task_leader = task::spawn(async move {
            loop {
                match rotor_leader.receive().await {
                    Ok(shred) => {
                        rotor_leader.forward(&shred).await.unwrap();
                    }
                    _ => continue,
                }
            }
        });

        tokio::time::sleep(Duration::from_millis(100)).await;

        // non-leader instances should have received all shreds via Rotor
        for i in 0..(count - 1) {
            assert_eq!(shreds_received[i as usize].lock().await.len(), TOTAL_SHREDS);
        }
        rotor_task_leader.abort();
        for task in rotor_tasks {
            task.abort();
        }
    }

    #[tokio::test]
    async fn two_instances() {
        test_rotor_dissemination(2, 3000).await
    }

    #[tokio::test]
    async fn many_instances() {
        test_rotor_dissemination(10, 3100).await
    }
}
