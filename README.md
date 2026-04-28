# QSMxT (Rust)

A command-line tool and interactive TUI for Quantitative Susceptibility Mapping (QSM) processing, built in Rust. QSMxT automates the full QSM pipeline on [BIDS](https://bids.neuroimaging.io/)-formatted neuroimaging datasets.

All algorithms are provided by [QSM.rs](https://github.com/astewartau/QSM.rs).

## Features

- **End-to-end QSM pipeline** -- masking, phase unwrapping, echo combination, background field removal, dipole inversion, and QSM referencing
- **Interactive TUI** -- configure and run pipelines with a terminal interface
- **BIDS-native** -- auto-discovers phase/magnitude pairs, reads JSON sidecars, writes BIDS-compliant derivatives to `derivatives/qsmxt.rs/`
- **10 inversion algorithms** -- RTS, TV, TKD, TSVD, TGV, Tikhonov, NLTV, MEDI, iLSQR, QSMART
- **Flexible masking** -- composable mask sections with threshold/BET generators and morphological refinements, OR'd together
- **Supplementary outputs** -- SWI, T2\* mapping, R2\* mapping, RSS-combined magnitude
- **Disk caching** -- intermediate results saved to disk; re-runs skip completed steps automatically
- **Memory-aware parallelism** -- concurrent processing with automatic memory limit detection
- **HPC support** -- SLURM job generation configurable from the TUI or CLI
- **Standalone commands** -- run individual processing steps directly on NIfTI files
- **Logging** -- full pipeline log saved to `qsmxt.log` in the output directory with version info

## Installation

### From release binaries

Download the latest binary for your platform from the [Releases](https://github.com/astewartau/qsmxt.rs/releases) page.

### From source

Requires Rust (edition 2021+).

```sh
cargo build --release
```

The binary is produced at `target/release/qsmxt`.

## Quick start

```sh
# Run the default pipeline (output goes to <bids_dir>/derivatives/qsmxt.rs/)
qsmxt run /path/to/bids

# Specify a separate output directory
qsmxt run /path/to/bids /path/to/output

# Dry run to preview what will be processed
qsmxt run /path/to/bids --dry

# Filter to specific subjects
qsmxt run /path/to/bids --subjects sub-01 sub-02

# Launch the interactive TUI
qsmxt tui
```

## Output structure

QSMxT writes outputs to `<output_dir>/derivatives/qsmxt.rs/` (or `<bids_dir>/derivatives/qsmxt.rs/` if no output directory is specified):

```
derivatives/qsmxt.rs/
  pipeline_config.toml          # Pipeline configuration used
  qsmxt.log                     # Full pipeline log with version info
  sub-01/
    anat/
      sub-01_Chimap.nii         # QSM map (referenced)
      sub-01_mask.nii           # Brain mask
      sub-01_magnitude.nii      # RSS-combined magnitude (homogeneity-corrected if enabled)
      sub-01_swi.nii            # SWI (if --do-swi)
      sub-01_minIP.nii          # SWI minimum intensity projection (if --do-swi)
      sub-01_T2starmap.nii      # T2* map (if --do-t2starmap)
      sub-01_R2starmap.nii      # R2* map (if --do-r2starmap)
```

## Interactive TUI

Launch with `qsmxt tui`. The TUI provides four tabs:

| Tab | Description |
|-----|-------------|
| **1: Input** | Set BIDS directory, output directory (optional), config file; browse and filter subjects/sessions/runs |
| **2: Pipeline** | Configure masking, unwrapping, background removal, inversion, and all algorithm parameters |
| **3: Supplementary** | Toggle SWI, T2\*/R2\* maps, configure SWI parameters |
| **4: Execution** | Switch between Local and SLURM mode; configure execution settings |

The Execution tab switches between **Local** mode (dry run, debug, thread count) and **SLURM** mode (account, partition, time limit, memory, CPUs, auto-submit).

### Keyboard shortcuts

| Key | Action |
|-----|--------|
| `1`-`4` | Switch tabs |
| `↑`/`↓` or `j`/`k` | Navigate fields |
| `←`/`→` | Adjust select values |
| `Enter`/`Space` | Edit text fields / toggle checkboxes |
| `r` | Reset focused field to default |
| `R` | Reset all fields on current tab |
| `d` | Delete mask refinement step |
| `Ctrl+↑`/`Ctrl+↓` | Reorder mask steps |
| `F5` | Run pipeline |
| `q` / `Esc` | Quit |

### Mask editor

The Pipeline tab includes an interactive mask editor with:

- **Presets**: Robust threshold, BET, Simple threshold, or Custom
- **Per-section structure**: Input source → Generator (threshold/BET) → Refinement steps
- **Multiple sections**: Add masks that are OR'd (combined) together
- **Refinement steps**: erode, dilate, close, fill-holes, gaussian -- add, delete, and reorder

A live command preview at the bottom shows the equivalent CLI command.

## Pipeline command (`run`)

```sh
qsmxt run <BIDS_DIR> [OUTPUT_DIR] [OPTIONS]
```

The output directory is optional -- if omitted, outputs go to `<BIDS_DIR>/derivatives/qsmxt.rs/`.

### Filtering

```
--subjects <SUB>...         Process only these subjects
--sessions <SES>...         Process only these sessions
--acquisitions <ACQ>...     Process only these acquisitions
--runs <RUN>...             Process only these runs
--num-echoes <N>            Limit number of echoes
```

### Algorithm selection

```
--qsm-algorithm <ALGO>     rts, tv, tkd, tsvd, tgv, tikhonov, nltv, medi, ilsqr, qsmart
--unwrapping-algorithm <A>  romeo, laplacian (default)
--bf-algorithm <ALGO>       vsharp (default), pdf, lbv, ismv, sharp
--qsm-reference <REF>      mean (default), none
--combine-phase <BOOL>      true (MCPC-3D-S, default), false (linear fit)
```

### Masking

```
--mask <SECTION>            Define a mask section (repeatable, OR'd together)
--mask-preset <PRESET>      Use a mask preset (robust-threshold or bet)
```

Format: `<input>,<generator>,<refinement1>,<refinement2>,...`

```sh
# Single robust threshold mask (default)
--mask phase-quality,threshold:otsu,dilate:2,fill-holes:0,erode:2

# BET mask
--mask magnitude,bet:0.5,erode:2

# Two masks combined (OR'd)
--mask phase-quality,threshold:otsu,dilate:2,erode:2 \
--mask magnitude,bet:0.5
```

**Input sources**: `phase-quality`, `magnitude`, `magnitude-first`, `magnitude-last`

**Generators**: `threshold:otsu`, `threshold:fixed:<value>`, `threshold:percentile:<value>`, `bet:<fractional_intensity>`

**Refinements**: `erode:<n>`, `dilate:<n>`, `close:<n>`, `fill-holes:<n>`, `gaussian:<sigma>`

### Supplementary outputs

```
--do-swi                    Compute susceptibility-weighted images
--do-t2starmap              Compute T2* map from multi-echo magnitude
--do-r2starmap              Compute R2* map from multi-echo magnitude
--inhomogeneity-correction  Apply B1 bias field correction to combined magnitude (on by default)
```

### Execution

```
--config <PATH>             Load pipeline configuration from TOML file
--n-procs <N>               Number of parallel threads
--force                     Re-run, ignoring cached pipeline state
--clean-intermediates       Remove intermediate files after completion
--dry                       Print processing plan without executing
--debug                     Enable debug logging
--mem-limit-gb <GB>         Memory limit for concurrent scheduling
```

### Algorithm parameters

Each algorithm has configurable parameters exposed as CLI flags (e.g. `--rts-delta`, `--tv-lambda`, `--medi-smv-radius`). Run `qsmxt run --help` for the full list. All defaults come from [QSM.rs](https://github.com/astewartau/QSM.rs).

## Other commands

| Command    | Description |
|------------|-------------|
| `init`     | Generate a default pipeline configuration file (TOML) |
| `validate` | Validate BIDS dataset structure for QSM processing |
| `slurm`    | Generate SLURM job scripts for HPC execution |
| `tui`      | Launch the interactive TUI |

## Standalone commands

Each command operates on individual NIfTI files:

| Command       | Description |
|---------------|-------------|
| `bet`         | Brain extraction |
| `mask`        | Binary mask creation (Otsu or manual threshold) |
| `unwrap`      | Phase unwrapping (ROMEO, Laplacian) |
| `bgremove`    | Background field removal (V-SHARP, PDF, LBV, iSMV, SHARP) |
| `invert`      | Dipole inversion (RTS, TV, TKD, TSVD, TGV, Tikhonov, NLTV, MEDI, iLSQR) |
| `swi`         | Susceptibility-weighted imaging with optional MIP |
| `r2star`      | R2\* mapping from multi-echo magnitude (ARLO) |
| `t2star`      | T2\* mapping from multi-echo magnitude |
| `homogeneity` | Inhomogeneity correction on magnitude data |
| `resample`    | Resample oblique volume to axial orientation |
| `dilate`      | Dilate a binary mask |
| `close`       | Morphological closing on a binary mask |
| `fill-holes`  | Fill holes in a binary mask |
| `smooth-mask` | Gaussian smooth a binary mask |
| `quality-map` | Compute ROMEO phase quality map |

## SLURM / HPC

Generate per-subject job scripts for cluster execution:

```sh
qsmxt slurm /path/to/bids \
  --account myaccount \
  --partition gpu \
  --time 02:00:00 \
  --mem 32 \
  --submit  # optionally auto-submit via sbatch
```

Or configure SLURM from the TUI by switching the Execution tab to SLURM mode.

## Configuration (TOML)

All pipeline settings can be specified in a TOML file. Generate a template with `qsmxt init`. The config file is overridden by CLI flags.

```sh
qsmxt init -o pipeline.toml                        # Generate template
qsmxt run /bids --config pipeline.toml             # Use it
```

## Algorithms

All QSM algorithms (inversion, background removal, phase unwrapping, masking, and utilities) are implemented in [QSM.rs](https://github.com/astewartau/QSM.rs). See the QSM.rs README for algorithm details, citations, parameter documentation, and default values.

## Project structure

```
src/
  main.rs           Entry point and CLI dispatch
  cli.rs            Argument definitions (clap)
  error.rs          Error types
  bids/
    discovery.rs    BIDS dataset scanning and run discovery
    entities.rs     BIDS filename entity parsing
    sidecar.rs      JSON sidecar reading
    derivatives.rs  BIDS derivative output paths
  pipeline/
    config.rs       Pipeline configuration and mask sections
    runner.rs       Core QSM processing pipeline
    phase.rs        Phase utilities (scaling, B0, unit conversion)
    graph.rs        Pipeline state and caching
    memory.rs       Memory estimation for concurrent scheduling
  tui/
    mod.rs          TUI event loop
    app.rs          Application state, pipeline form, mask editor
    ui.rs           Rendering (ratatui)
    command.rs      CLI command string and args generation
  executor/
    local.rs        Parallel local execution with memory awareness
    slurm.rs        SLURM script generation
  commands/         Standalone command implementations
```

## License

See the repository root for license information.
