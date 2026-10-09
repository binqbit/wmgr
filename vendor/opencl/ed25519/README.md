# Ed25519 OpenCL arithmetic

`preamble.cl`, `fe.cl`, `precomp.cl`, `ge.cl`, and `sha512.cl` originate from
[cavemanloverboy/vanity](https://github.com/cavemanloverboy/vanity), revision
`a7ad53630be9b11b895eeb0aee3bc9361a7f46ce`, under Apache-2.0 (see `LICENSE`).
Copyright 2024 Cavey Cool and contributors. The arithmetic is an OpenCL port
of the ref10 Ed25519 implementation. The upstream revision is pinned so builds
do not download or execute remote source.

wmgr changes:

- `ge.cl`: remove unused comb/batched multiplication functions and replace the
  secret-indexed base-point lookup with a fixed table scan and conditional moves.
- `derive.cl`: new bounded entry point accepting independent 32-byte seeds from
  the host. It hashes each seed with SHA-512, clamps the Ed25519 scalar, multiplies
  by the base point, and returns the 32-byte public key. There is no GPU RNG.

The Rust adapter embeds these files, validates the device with RFC 8032 vectors
and Solana SDK comparisons, and wipes the seed buffer after every batch.
