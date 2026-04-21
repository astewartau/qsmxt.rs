# QSMxT.rs Feature Checklist

Features for the Rust rewrite of QSMxT. Checked items are implemented or targeted for implementation.

## Core Pipeline

- [x] Load NIfTI phase and magnitude data
- [x] Scale phase to [-pi, pi]
- [x] Create brain mask
- [x] Mask erosion
- [x] Phase unwrapping
- [x] Multi-echo phase combination (MCPC-3D-S)
- [x] Background field removal
- [x] Dipole inversion (QSM reconstruction)
- [x] QSM referencing (mean subtraction)
- [x] SWI computation + minimum intensity projection

## QSM Algorithms

- [x] RTS (Regularized Total Least Squares)
- [x] TV (Total Variation ADMM)
- [x] TKD (Threshold K-space Division)
- [x] TGV (Total Generalized Variation, single-step)

## Phase Unwrapping

- [x] ROMEO (Region-based Order Mapping)
- [x] Laplacian (FFT-based)

## Background Field Removal

- [x] V-SHARP
- [x] PDF (Projection onto Dipole Field)
- [x] LBV (Laplacian Boundary Value)
- [x] iSMV (Iterative Susceptibility Mask-based)

## Masking

- [x] BET (Brain Extraction Tool, icosphere-based)
- [x] Otsu thresholding
- [x] Gaussian mask filling — via `--mask-op gaussian:4.0` or `qsmxt smooth-mask`
- [x] Morphological mask filling — via `--mask-op close:1`, `fill-holes:1000`, `dilate:1` or standalone commands
- [x] Phase quality mapping (ROMEO-based quality maps for masking input) — via `--masking-input phase-quality`

## Additional Processing

- [x] T2* mapping (multi-echo relaxation fitting) — uses QSM.rs ARLO algorithm via `--do-t2starmap`
- [x] R2* mapping (multi-echo decay rate fitting) — uses QSM.rs ARLO algorithm via `--do-r2starmap`
- [x] Inhomogeneity correction (B1 field correction before masking) — uses QSM.rs `makehomogeneous` via `--inhomogeneity-correction`
- [x] Obliquity resampling (resample oblique acquisitions to axial) — via `--obliquity-threshold`, warns for axial-only algorithms when disabled

## Pipeline Configuration

- [x] TOML-based pipeline config files
- [x] Pipeline presets (GRE, EPI, BET, Fast, Body)
- [x] CLI overrides for all parameters
- [x] Config validation

## BIDS Support

- [x] BIDS dataset discovery and scanning
- [x] Entity parsing (sub, ses, acq, run, echo)
- [x] JSON sidecar reading (echo times, B0 field strength)
- [x] BIDS derivatives output structure
- [x] Filtering by subject, session, acquisition, run, echo count

## Execution

- [x] Parallel processing (Rayon thread pool)
- [x] Memory-aware concurrency throttling
- [x] Memory estimation per run
- [x] SLURM job script generation
- [ ] PBS cluster support — qsmxt.rs only; mirror SLURM generator
- [x] Dry-run mode

## CLI Commands

- [x] `run` - Full pipeline execution
- [x] `init` - Generate config file from preset
- [x] `validate` - BIDS dataset validation
- [x] `presets` - List/show pipeline presets
- [x] `slurm` - Generate SLURM scripts
- [x] `bet` - Standalone brain extraction
- [x] `mask` - Standalone mask creation
- [x] `unwrap` - Standalone phase unwrapping
- [x] `bgremove` - Standalone background field removal
- [x] `invert` - Standalone dipole inversion
- [x] `swi` - Standalone SWI computation
- [x] `r2star` - Standalone R2* mapping
- [x] `t2star` - Standalone T2* mapping
- [x] `homogeneity` - Standalone inhomogeneity correction
- [x] `resample` - Standalone obliquity resampling to axial
- [x] `quality-map` - Standalone ROMEO phase quality map computation
- [x] `dilate` - Standalone mask dilation
- [x] `close` - Standalone morphological closing
- [x] `fill-holes` - Standalone hole filling
- [x] `smooth-mask` - Standalone Gaussian mask smoothing

## User Interface

- [x] CLI (clap-based)
- [ ] TUI (ratatui-based, in progress)

## Miscellaneous

- [ ] Citation tracking (auto-generate citations for used algorithms) — QSM.rs has DOIs in source docs but no runtime collector; needs a registry in qsmxt.rs
