# QSMxT (Rust)

A command-line tool for Quantitative Susceptibility Mapping (QSM) processing, built in Rust. QSMxT automates the full QSM pipeline on [BIDS](https://bids.neuroimaging.io/)-formatted neuroimaging datasets, from phase unwrapping through to dipole inversion.

## Features

- **End-to-end QSM pipeline** -- mask creation, phase unwrapping, echo combination, background field removal, and dipole inversion
- **BIDS-native** -- automatically discovers phase/magnitude pairs, reads JSON sidecars, and writes BIDS-compliant derivatives
- **Multiple algorithms** -- choose from several algorithms at each processing stage
- **Pipeline presets** -- built-in configurations for common acquisition types (GRE, EPI, etc.)
- **Parallel execution** -- processes multiple acquisitions concurrently via Rayon
- **HPC support** -- generates and optionally submits SLURM job scripts for cluster execution
- **Standalone commands** -- run individual processing steps (BET, masking, unwrapping, background removal, dipole inversion, SWI) directly on NIfTI files

## Installation

Requires Rust (edition 2021+) and the [`qsm-core`](../../QSM.rs) library.

```sh
cargo build --release
```

The binary is produced at `target/release/qsmxt`.

## Quick start

```sh
# Run the default GRE pipeline
qsmxt run /path/to/bids /path/to/output --preset gre

# Dry run to preview what will be processed
qsmxt run /path/to/bids /path/to/output --preset gre --dry

# Filter to specific subjects
qsmxt run /path/to/bids /path/to/output --preset gre --subjects sub-01 sub-02
```

## Commands

### Pipeline

| Command    | Description |
|------------|-------------|
| `run`      | Run the full QSM pipeline on a BIDS dataset |
| `init`     | Generate a pipeline configuration file (TOML) |
| `validate` | Validate BIDS dataset structure for QSM processing |
| `presets`  | List or show details for pipeline presets |
| `slurm`    | Generate SLURM job scripts for HPC execution |

### Standalone processing

Each command operates on individual NIfTI files:

| Command    | Description |
|------------|-------------|
| `bet`      | Brain extraction |
| `mask`     | Binary mask creation via thresholding (Otsu or manual) |
| `unwrap`   | Phase unwrapping (Laplacian, ROMEO) |
| `bgremove` | Background field removal (V-SHARP, PDF, LBV, iSMV) |
| `invert`   | Dipole inversion / QSM (RTS, TV, TKD, TGV) |
| `swi`      | Susceptibility-weighted imaging (with optional MIP) |

## Presets

| Preset | Use case |
|--------|----------|
| `gre`  | Standard gradient-echo acquisitions |
| `epi`  | Echo-planar imaging acquisitions |
| `bet`  | Pipeline using BET for masking |
| `fast` | Fast processing with simpler algorithms |
| `body` | Body / non-brain applications |

Generate a TOML config from a preset for further customisation:

```sh
qsmxt init --preset gre -o pipeline.toml
qsmxt run /path/to/bids /path/to/output --config pipeline.toml
```

## Algorithm options

### QSM (dipole inversion)
- **RTS** -- Rapid Two-Step (default for most presets)
- **TV** -- Total Variation
- **TKD** -- Thresholded K-space Division
- **TGV** -- Total Generalised Variation

### Phase unwrapping
- **Laplacian** (default)
- **ROMEO**

### Background field removal
- **V-SHARP** (default)
- **PDF**
- **LBV**
- **iSMV**

### Masking
- **BET** -- brain extraction
- **Threshold** -- Otsu or manual thresholding

## SLURM / HPC

Generate per-subject job scripts for cluster execution:

```sh
qsmxt slurm /path/to/bids /path/to/output \
  --account myaccount \
  --partition gpu \
  --preset gre \
  --time 02:00:00 \
  --mem 32 \
  --submit  # optionally auto-submit via sbatch
```

## Project structure

```
src/
  main.rs           # Entry point and CLI dispatch
  cli.rs            # Argument definitions (clap)
  error.rs          # Error types
  bids/
    discovery.rs    # BIDS dataset scanning and run discovery
    entities.rs     # BIDS filename entity parsing
    sidecar.rs      # JSON sidecar reading
    derivatives.rs  # BIDS derivative output paths
  pipeline/
    config.rs       # Pipeline configuration and presets
    runner.rs       # Core QSM processing pipeline
    phase.rs        # Phase utilities (scaling, B0, unit conversion)
  executor/
    local.rs        # Parallel local execution
    slurm.rs        # SLURM script generation
  commands/         # Subcommand implementations
```

## License

See the repository root for license information.
