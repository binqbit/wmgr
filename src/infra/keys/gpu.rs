//! GPU acceleration for Solana key generation. Entropy stays on the host;
//! OpenCL only derives public keys from independent, full-length seeds.

use std::{env, fs, path::PathBuf, ptr, sync::OnceLock};

use anyhow::{anyhow, bail, Context as _, Result};
use libloading::Library;
use opencl3::{
    command_queue::CommandQueue,
    context::Context,
    device::{get_all_devices, Device, CL_DEVICE_TYPE_GPU},
    kernel::Kernel,
    memory::{Buffer, ClMem, CL_MEM_READ_ONLY, CL_MEM_WRITE_ONLY},
    program::Program,
    types::CL_BLOCKING,
};
use solana_keypair::Keypair;
use solana_sdk::signature::Signer;

const BATCH_SIZE: usize = 1024;
const SEED_BYTES: usize = 32;
const KERNEL_SOURCE: &str = concat!(
    include_str!("../../../vendor/opencl/ed25519/preamble.cl"),
    "\n",
    include_str!("../../../vendor/opencl/ed25519/fe.cl"),
    "\n",
    include_str!("../../../vendor/opencl/ed25519/precomp.cl"),
    "\n",
    include_str!("../../../vendor/opencl/ed25519/ge.cl"),
    "\n",
    include_str!("../../../vendor/opencl/ed25519/sha512.cl"),
    "\n",
    include_str!("../../../vendor/opencl/ed25519/derive.cl"),
);

pub struct OpenClGenerator {
    seeds: Buffer<u8>,
    public_keys: Buffer<u8>,
    kernel: Kernel,
    queue: CommandQueue,
    // Keep the context alive until the other resources have been released.
    _context: Context,
    device_name: String,
}

impl OpenClGenerator {
    pub fn new() -> Result<Self> {
        preload_opencl_loader();
        let mut devices: Vec<_> = get_all_devices(CL_DEVICE_TYPE_GPU)
            .context("OpenCL GPU discovery failed")?
            .into_iter()
            .map(Device::new)
            .filter(|device| {
                device.available().unwrap_or(false) && device.compiler_available().unwrap_or(false)
            })
            .collect();
        devices.sort_by_key(|device| {
            std::cmp::Reverse(
                u64::from(device.max_compute_units().unwrap_or(0))
                    * u64::from(device.max_clock_frequency().unwrap_or(1)),
            )
        });
        if devices.is_empty() {
            bail!("No available OpenCL GPU with a compiler was found");
        }

        let mut failures = Vec::new();
        for device in devices {
            let name = device.name().unwrap_or_else(|_| "OpenCL GPU".into());
            match Self::for_device(&device, name.clone()) {
                Ok(generator) => return Ok(generator),
                Err(error) => failures.push(format!("{name}: {error:#}")),
            }
        }
        bail!("No usable OpenCL GPU: {}", failures.join("; "))
    }

    fn for_device(device: &Device, device_name: String) -> Result<Self> {
        let context = Context::from_device(device).context("Creating OpenCL context")?;
        // CL 1.2 keeps this compatible with NVIDIA, AMD, Intel, and macOS drivers.
        #[allow(deprecated)]
        let queue =
            CommandQueue::create_default(&context, 0).context("Creating OpenCL command queue")?;
        let program =
            Program::create_and_build_from_source(&context, KERNEL_SOURCE, "-cl-std=CL1.2")
                .map_err(|error| anyhow!("Building Ed25519 OpenCL kernel: {error}"))?;
        let kernel = Kernel::create(&program, "wmgr_ed25519_public_keys")
            .context("Creating Ed25519 OpenCL kernel")?;
        // No host pointer is passed, and all accesses are bounded by BATCH_SIZE.
        let seeds = unsafe {
            Buffer::create(
                &context,
                CL_MEM_READ_ONLY,
                BATCH_SIZE * SEED_BYTES,
                ptr::null_mut(),
            )?
        };
        let public_keys = unsafe {
            Buffer::create(
                &context,
                CL_MEM_WRITE_ONLY,
                BATCH_SIZE * SEED_BYTES,
                ptr::null_mut(),
            )?
        };
        let mut generator = Self {
            seeds,
            public_keys,
            kernel,
            queue,
            _context: context,
            device_name,
        };
        generator
            .self_test()
            .context("OpenCL Ed25519 self-test failed")?;
        Ok(generator)
    }

    pub fn device_name(&self) -> &str {
        &self.device_name
    }

    pub fn batch_size(&self) -> usize {
        BATCH_SIZE
    }

    /// Derive a bounded batch. Callers generate seeds with a host CSPRNG and
    /// independently verify any matching public key with the Solana SDK.
    pub fn public_keys(&mut self, seeds: &[[u8; 32]]) -> Result<Vec<[u8; 32]>> {
        if seeds.len() > BATCH_SIZE {
            bail!("OpenCL batch exceeds {BATCH_SIZE} seeds");
        }
        if seeds.is_empty() {
            return Ok(Vec::new());
        }
        let result = self.derive_batch(seeds);
        // Attempt cleanup on both success and failure, including failed launches.
        let cleanup = self.wipe_seeds();
        match result {
            Err(error) => Err(error),
            Ok(keys) => {
                cleanup.context("Clearing OpenCL seed buffer")?;
                Ok(keys)
            }
        }
    }

    fn derive_batch(&mut self, seeds: &[[u8; 32]]) -> Result<Vec<[u8; 32]>> {
        let mut public_keys = vec![[0u8; 32]; seeds.len()];
        let count = seeds.len() as u32;
        // Blocking writes/reads and an in-order queue ensure slices remain alive
        // until all host accesses finish. Each work-item owns one 32-byte slot.
        unsafe {
            self.queue.enqueue_write_buffer(
                &mut self.seeds,
                CL_BLOCKING,
                0,
                seeds.as_flattened(),
                &[],
            )?;
            // Use fallible APIs: ExecuteKernel's convenience setters panic on
            // driver errors, which would bypass normal cleanup and CPU fallback.
            self.kernel.set_arg(0, &self.seeds.get())?;
            self.kernel.set_arg(1, &self.public_keys.get())?;
            self.kernel.set_arg(2, &count)?;
            let global_work_size = [seeds.len()];
            self.queue
                .enqueue_nd_range_kernel(
                    self.kernel.get(),
                    1,
                    ptr::null(),
                    global_work_size.as_ptr(),
                    ptr::null(),
                    &[],
                )?
                .wait()?;
            self.queue.enqueue_read_buffer(
                &self.public_keys,
                CL_BLOCKING,
                0,
                public_keys.as_flattened_mut(),
                &[],
            )?;
        }
        Ok(public_keys)
    }

    fn wipe_seeds(&mut self) -> Result<()> {
        // OpenCL copies the pattern before returning; the in-order queue makes
        // the fill follow any preceding operation, even a failed host read.
        unsafe {
            self.queue
                .enqueue_fill_buffer(&mut self.seeds, &[0u8], 0, BATCH_SIZE * SEED_BYTES, &[])?
                .wait()?;
        }
        Ok(())
    }

    fn self_test(&mut self) -> Result<()> {
        // RFC 8032 section 7.1, plus a multi-item batch compared against the
        // established SDK. These are public test vectors, never wallet entropy.
        let first: [u8; 32] =
            hex::decode("9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60")?
                .try_into()
                .map_err(|_| anyhow!("Invalid embedded Ed25519 test seed"))?;
        let second: [u8; 32] =
            hex::decode("4ccd089b28ff96da9db6c346ec114e0f5b8a319f35aba624da8cf6ed4fb8a6fb")?
                .try_into()
                .map_err(|_| anyhow!("Invalid embedded Ed25519 test seed"))?;
        let seeds = [first, second, [0; 32], [255; 32], [42; 32]];
        let public_keys = self.public_keys(&seeds)?;
        if hex::encode(public_keys[0])
            != "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a"
            || hex::encode(public_keys[1])
                != "3d4017c3e843895a92b70aa74d1b7ebc9c982ccf2ec4968cc0cd55f12af4660c"
        {
            bail!("GPU returned an incorrect RFC 8032 Ed25519 public key");
        }
        for (seed, public_key) in seeds.into_iter().zip(public_keys) {
            if Keypair::new_from_array(seed).pubkey().to_bytes() != public_key {
                bail!("GPU Ed25519 result differs from the Solana SDK");
            }
        }
        Ok(())
    }
}

impl Drop for OpenClGenerator {
    fn drop(&mut self) {
        // A failed device/driver can prevent erasure, so this is best effort.
        let _ = self.wipe_seeds();
    }
}

fn preload_opencl_loader() {
    static LOADER: OnceLock<Option<Library>> = OnceLock::new();
    LOADER.get_or_init(|| {
        // Respect explicit configuration, including a deliberately invalid path
        // used to test fallback. opencl3 handles its own standard loader names.
        if env::var_os("OPENCL_DYLIB_PATH").is_some() || !cfg!(target_os = "linux") {
            return None;
        }
        // On Linux an already loaded SONAME is reusable by opencl3, so no
        // process-wide environment mutation is needed in a multithreaded CLI.
        for path in [
            PathBuf::from("libOpenCL.so.1"),
            PathBuf::from("/run/opengl-driver/lib/libOpenCL.so.1"),
            PathBuf::from("/usr/lib/libOpenCL.so.1"),
            PathBuf::from("/usr/local/lib/libOpenCL.so.1"),
        ] {
            // Loading the locally installed OpenCL driver is necessary for GPU
            // execution. Library ownership is retained for the process lifetime.
            if let Ok(library) = unsafe { Library::new(path) } {
                return Some(library);
            }
        }
        let mut candidates: Vec<_> = fs::read_dir("/nix/store")
            .ok()?
            .flatten()
            .filter(|entry| {
                let name = entry.file_name();
                let name = name.to_string_lossy();
                name.contains("-ocl-icd-") || name.contains("-opencl-icd-loader-")
            })
            .map(|entry| entry.path().join("lib/libOpenCL.so.1"))
            .collect();
        candidates.sort();
        candidates
            .into_iter()
            .find_map(|path| unsafe { Library::new(path).ok() })
    });
}
