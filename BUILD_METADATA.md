# ROHO (RH) Build Metadata — Consensus Freeze Record

Generated: 2026-01-08T19:06:11+05:30

## Build Environment

| Item                  | Value                                                            |
| --------------------- | ---------------------------------------------------------------- |
| Rust Compiler         | rustc 1.92.0 (ded5c06cf 2025-12-08)                              |
| Cargo.lock SHA256     | c637eb355dc7997e99602654b44201a13e378a3041dc592ebc6356238f989a3f |
| rh-node Binary SHA256 | 35dc9cdb80ff1c5764d652e4cc97ae99277e1ded27d046fd6e182fff36ccb288 |

## Git Repository

**Status**: Not yet initialized

To complete freeze:

```bash
cd /Users/ranjeetharishchandre/Documents/Personal/rh-coin/rh-core
git init
git add .
git commit -S -m "ROHO v1.0 - Consensus Freeze"
git tag -s roho-v1.0-final -m "Final consensus freeze"
```

## Constitution Hash

To compute:

```bash
shasum -a 256 RH_CONSTITUTION.txt
```

## Genesis Block

**Hash**: `3153db7f3b03eb371f2227bdb8464626f41399de839dd739c77b6c71bc85d623`
**Merkle Root**: `6ddfb0aee2ba220015432f825e7992bcc52abe8d05677c37497fa8fdeb534172`
**Timestamp**: `1736339922`
**Founder Address**: `RH2Q3hRrvJ1MZFFW7LYbUghLCKEUjCHZWXU`
**Allocation**: `10,000,000 RH`

> **VERIFIED**: Byte-for-byte reproducibility confirmed.

## Verification Commands

```bash
# Verify Cargo.lock hash
shasum -a 256 Cargo.lock

# Verify Rust version
rustc --version

# Run all tests
cargo test

# Build reproducible binary
cargo build --release --locked
shasum -a 256 target/release/rh-node
```

## Freeze Rules

> **NO CHANGES PERMITTED AFTER THIS POINT**
>
> - No new features
> - No refactors
> - No "cleanups"
> - No dependency updates
>
> Any modification invalidates this freeze.
