# QSMxT (Rust)

A command-line tool and interactive TUI for Quantitative Susceptibility Mapping (QSM) processing, built in Rust. QSMxT automates the full QSM pipeline on [BIDS](https://bids.neuroimaging.io/)-formatted neuroimaging datasets.

All algorithms are provided by [QSM.rs](https://github.com/astewartau/QSM.rs).

## Features

- **End-to-end QSM pipeline** -- masking, phase unwrapping, echo combination, background field removal, dipole inversion, and QSM referencing
- **Interactive TUI** -- configure and run pipelines with a terminal interface
- **BIDS-native** -- auto-discovers phase/magnitude pairs, reads JSON sidecars, writes BIDS-compliant derivatives
- **10 inversion algorithms** -- RTS, TV, TKD, TSVD, TGV, Tikhonov, NLTV, MEDI, iLSQR, QSMART
- **Flexible masking** -- composable mask sections with threshold/BET generators and morphological refinements, OR'd together
- **Supplementary outputs** -- SWI, T2\* mapping, R2\* mapping
- **Pipeline presets** -- built-in configurations for GRE, EPI, BET, fast, and body applications
- **Disk caching** -- intermediate results saved to disk; re-runs skip completed steps automatically
- **Memory-aware parallelism** -- concurrent processing with automatic memory limit detection
- **HPC support** -- generates and optionally submits SLURM job scripts
- **Standalone commands** -- run individual processing steps directly on NIfTI files

## Installation

Requires Rust (edition 2021+).

```sh
cargo build --release
```

The binary is produced at `target/release/qsmxt`.

## Quick start

```sh
# Run the default pipeline
qsmxt run /path/to/bids /path/to/output

# Use a preset
qsmxt run /path/to/bids /path/to/output --preset gre

# Dry run to preview what will be processed
qsmxt run /path/to/bids /path/to/output --dry

# Filter to specific subjects
qsmxt run /path/to/bids /path/to/output --subjects sub-01 sub-02

# Launch the interactive TUI
qsmxt tui
```

## Interactive TUI

Launch with `qsmxt tui`. The TUI provides four tabs for configuring and running pipelines:

| Tab | Description |
|-----|-------------|
| **Input/Output** | Set BIDS directory, output directory, preset, and config file |
| **Filters** | Browse the BIDS tree, select/deselect subjects, sessions, and runs |
| **Pipeline** | Configure all pipeline settings: masking, unwrapping, BG removal, inversion, and their parameters |
| **Execution** | Toggle SWI, T2\*/R2\*, dry run, debug mode, and thread count |

### Keyboard shortcuts

| Key | Action |
|-----|--------|
| `1`-`4` | Switch tabs |
| `↑`/`↓` or `j`/`k` | Navigate fields |
| `←`/`→` | Adjust select values |
| `Enter` | Edit text fields / toggle checkboxes |
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
qsmxt run <BIDS_DIR> <OUTPUT_DIR> [OPTIONS]
```

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
--unwrapping-algorithm <A>  romeo (default), laplacian
--bf-algorithm <ALGO>       vsharp (default), pdf, lbv, ismv, sharp
--qsm-reference <REF>      mean (default), none
--combine-phase <BOOL>      true (MCPC-3D-S, default), false (linear fit)
```

### Masking

```
--mask <SECTION>            Define a mask section (repeatable, OR'd together)
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
--inhomogeneity-correction  Apply B1 bias field correction (on by default)
```

### Execution

```
--preset <PRESET>           Use a pipeline preset (gre, epi, bet, fast, body)
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

## Other pipeline commands

| Command    | Description |
|------------|-------------|
| `init`     | Generate a pipeline configuration file (TOML) from a preset |
| `validate` | Validate BIDS dataset structure for QSM processing |
| `presets`  | List or show details for pipeline presets |
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

## Presets

| Preset | Description |
|--------|-------------|
| `gre`  | Standard gradient-echo acquisitions (default) |
| `epi`  | Echo-planar imaging acquisitions |
| `bet`  | BET-based masking |
| `fast` | Fast processing |
| `body` | Body / non-brain applications (TGV single-step) |

Generate a TOML config from a preset for further customisation:

```sh
qsmxt init --preset gre -o pipeline.toml
qsmxt run /path/to/bids /path/to/output --config pipeline.toml
```

## Configuration (TOML)

All pipeline settings can be specified in a TOML file. Generate a template with `qsmxt init`. The config file overrides preset defaults, and CLI flags override the config file.

```sh
qsmxt init --preset gre -o pipeline.toml   # Generate template
qsmxt run /bids /out --config pipeline.toml # Use it
```

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
    config.rs       Pipeline configuration, presets, and mask sections
    runner.rs       Core QSM processing pipeline
    phase.rs        Phase utilities (scaling, B0, unit conversion)
    graph.rs        Pipeline state and caching
    memory.rs       Memory estimation for concurrent scheduling
  tui/
    mod.rs          TUI event loop
    app.rs          Application state, pipeline form, mask editor
    ui.rs           Rendering (ratatui)
    command.rs      CLI command string generation
    widgets.rs      Reusable TUI widgets
  executor/
    local.rs        Parallel local execution with memory awareness
    slurm.rs        SLURM script generation
  commands/         Standalone command implementations
```

## License

See the repository root for license information.
