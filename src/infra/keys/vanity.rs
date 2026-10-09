use std::time::{Duration, Instant};

use anyhow::{anyhow, bail, Context, Result};
use rand::{rngs::OsRng, RngCore};
use rayon::prelude::*;
use solana_keypair::Keypair;
use solana_sdk::{pubkey::Pubkey, signature::Signer};
use zeroize::Zeroizing;

use super::{generate::random_keypair, gpu::OpenClGenerator};
use crate::core::vanity::AddressPattern;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    Auto,
    Cpu,
    Gpu,
}

pub struct SearchOptions {
    pub backend: Backend,
    pub threads: usize,
    pub max_attempts: Option<u64>,
}

pub struct SearchResult {
    pub keypair: Keypair,
    pub attempts: u64,
    pub backend: String,
}

pub enum SearchProgress {
    Backend(String),
    Fallback(String),
    Attempts(u64),
}

pub fn search(
    pattern: &AddressPattern,
    options: &SearchOptions,
    mut progress: impl FnMut(SearchProgress),
) -> Result<SearchResult> {
    if options.threads == 0 || options.max_attempts == Some(0) {
        bail!("Threads and maximum attempts must be greater than zero");
    }
    let mut gpu = if options.backend == Backend::Cpu {
        None
    } else {
        match OpenClGenerator::new() {
            Ok(gpu) => Some(gpu),
            Err(error) if options.backend == Backend::Auto => {
                progress(SearchProgress::Fallback(format!("{error:#}")));
                None
            }
            Err(error) => bail!("GPU backend unavailable: {error:#}"),
        }
    };
    let mut pool = None;
    let mut backend = String::new();
    let mut attempts = 0u64;
    let mut last_progress = Instant::now();
    loop {
        if options.max_attempts.is_some_and(|max| attempts >= max) {
            bail!("No matching address found after {attempts} attempts; no wallet saved");
        }
        let batch_size = match gpu.as_ref() {
            Some(gpu) => gpu.batch_size(),
            None => options.threads.saturating_mul(64).clamp(64, 4096),
        };
        if batch_size == 0 {
            bail!("Search backend returned an empty batch size");
        }
        let count = options
            .max_attempts
            .map(|max| (max - attempts).min(batch_size as u64) as usize)
            .unwrap_or(batch_size);
        let found = if let Some(device) = gpu.as_mut() {
            if backend.is_empty() {
                backend = format!("GPU ({})", device.device_name());
                progress(SearchProgress::Backend(backend.clone()));
            }
            let mut seeds = Zeroizing::new(vec![[0u8; 32]; count]);
            for seed in seeds.iter_mut() {
                OsRng
                    .try_fill_bytes(seed)
                    .context("OS cryptographic randomness unavailable")?;
            }
            match gpu_batch(device, &seeds, pattern) {
                Ok(found) => found,
                Err(error) if options.backend == Backend::Auto => {
                    progress(SearchProgress::Fallback(format!("{error:#}")));
                    gpu = None;
                    backend.clear();
                    // Failed batches do not count as searched candidates.
                    continue;
                }
                Err(error) => bail!("GPU search failed: {error:#}"),
            }
        } else {
            if pool.is_none() {
                pool = Some(
                    rayon::ThreadPoolBuilder::new()
                        .num_threads(options.threads)
                        .build()
                        .context("Failed to start CPU search workers")?,
                );
                backend = format!(
                    "CPU ({} threads)",
                    pool.as_ref().unwrap().current_num_threads()
                );
                progress(SearchProgress::Backend(backend.clone()));
            }
            cpu_batch(pool.as_ref().unwrap(), count, pattern)?
        };
        attempts = attempts
            .checked_add(count as u64)
            .ok_or_else(|| anyhow!("Search attempt counter exhausted"))?;
        if let Some(keypair) = found {
            return Ok(SearchResult {
                keypair,
                attempts,
                backend,
            });
        }
        if last_progress.elapsed() >= Duration::from_secs(1) {
            progress(SearchProgress::Attempts(attempts));
            last_progress = Instant::now();
        }
    }
}

fn cpu_batch(
    pool: &rayon::ThreadPool,
    count: usize,
    pattern: &AddressPattern,
) -> Result<Option<Keypair>> {
    let candidates: Result<Vec<Option<Keypair>>> = pool.install(|| {
        (0..count)
            .into_par_iter()
            .map(|_| {
                let keypair = random_keypair()?;
                Ok(pattern
                    .matches(&keypair.pubkey().to_string())
                    .then_some(keypair))
            })
            .collect()
    });
    Ok(candidates?.into_iter().flatten().next())
}

fn gpu_batch(
    gpu: &mut OpenClGenerator,
    seeds: &[[u8; 32]],
    pattern: &AddressPattern,
) -> Result<Option<Keypair>> {
    let public_keys = gpu.public_keys(seeds)?;
    if public_keys.len() != seeds.len() {
        bail!("GPU returned an incorrect number of public keys");
    }
    // Verify a sample from every batch, as well as every matching candidate, with the SDK.
    if let (Some(seed), Some(public_key)) = (seeds.first(), public_keys.first()) {
        if Keypair::new_from_array(*seed).pubkey().to_bytes() != *public_key {
            bail!("GPU public key verification failed");
        }
    }
    for (seed, public_key) in seeds.iter().zip(public_keys) {
        let address = Pubkey::new_from_array(public_key).to_string();
        if pattern.matches(&address) {
            let keypair = Keypair::new_from_array(*seed);
            if keypair.pubkey().to_bytes() != public_key {
                bail!("GPU match failed Solana keypair verification");
            }
            return Ok(Some(keypair));
        }
    }
    Ok(None)
}
