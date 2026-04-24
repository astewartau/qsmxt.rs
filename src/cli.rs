use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "qsmxt",
    version,
    about = "QSMxT: Quantitative Susceptibility Mapping tool (Rust)"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
#[allow(clippy::large_enum_variant)]
pub enum Command {
    /// Run the full QSM pipeline on a BIDS dataset
    Run(RunArgs),
    /// Generate a pipeline configuration file
    Init(InitArgs),
    /// Validate BIDS dataset structure for QSM processing
    Validate(ValidateArgs),
    /// List or show pipeline presets
    Presets(PresetsArgs),
    /// Generate SLURM job scripts for HPC execution
    Slurm(SlurmArgs),
    /// Brain extraction (NIfTI in/out)
    Bet(BetArgs),
    /// Create a binary mask via thresholding (NIfTI in/out)
    Mask(MaskArgs),
    /// Phase unwrapping (NIfTI in/out)
    Unwrap(UnwrapArgs),
    /// Background field removal (NIfTI in/out)
    Bgremove(BgremoveArgs),
    /// Dipole inversion (NIfTI in/out)
    Invert(InvertArgs),
    /// Susceptibility-weighted imaging (NIfTI in/out)
    Swi(SwiArgs),
    /// R2* mapping from multi-echo magnitude data (NIfTI in/out)
    R2star(R2starArgs),
    /// T2* mapping from multi-echo magnitude data (NIfTI in/out)
    T2star(T2starArgs),
    /// Inhomogeneity correction on magnitude data (NIfTI in/out)
    Homogeneity(HomogeneityArgs),
    /// Resample oblique volume to axial orientation (NIfTI in/out)
    Resample(ResampleArgs),
    /// Dilate a binary mask (NIfTI in/out)
    Dilate(DilateArgs),
    /// Morphological closing on a binary mask (NIfTI in/out)
    Close(CloseArgs),
    /// Fill holes in a binary mask (NIfTI in/out)
    FillHoles(FillHolesArgs),
    /// Gaussian smooth a binary mask (NIfTI in/out)
    SmoothMask(SmoothMaskArgs),
    /// Compute ROMEO phase quality map (NIfTI in/out)
    #[command(name = "quality-map")]
    QualityMap(QualityMapArgs),
    /// Launch interactive TUI for pipeline configuration
    Tui,
}

// ─── Pipeline commands ───

#[derive(Parser, Debug)]
pub struct RunArgs {
    /// Input BIDS directory
    pub bids_dir: PathBuf,

    /// Output derivatives directory
    pub output_dir: PathBuf,

    /// Use a premade pipeline preset
    #[arg(long, value_enum)]
    pub preset: Option<Preset>,

    /// Pipeline configuration file (TOML)
    #[arg(long)]
    pub config: Option<PathBuf>,

    // Filters
    /// Process only these subjects (e.g., sub-01 sub-02)
    #[arg(long, num_args = 1..)]
    pub subjects: Option<Vec<String>>,

    /// Process only these sessions
    #[arg(long, num_args = 1..)]
    pub sessions: Option<Vec<String>>,

    /// Process only these acquisitions
    #[arg(long, num_args = 1..)]
    pub acquisitions: Option<Vec<String>>,

    /// Process only these runs
    #[arg(long, num_args = 1..)]
    pub runs: Option<Vec<String>>,

    /// Limit number of echoes to process
    #[arg(long)]
    pub num_echoes: Option<usize>,

    // Algorithm overrides
    /// QSM algorithm
    #[arg(long, value_enum)]
    pub qsm_algorithm: Option<QsmAlgorithmArg>,

    /// Unwrapping algorithm
    #[arg(long, value_enum)]
    pub unwrapping_algorithm: Option<UnwrapAlgorithmArg>,

    /// Background field removal algorithm
    #[arg(long, value_enum)]
    pub bf_algorithm: Option<BfAlgorithmArg>,

    /// Masking algorithm
    #[arg(long, value_enum)]
    pub masking_algorithm: Option<MaskAlgorithmArg>,

    /// Masking input type
    #[arg(long, value_enum)]
    pub masking_input: Option<MaskInputArg>,

    // Parameter overrides
    /// Combine multi-echo phase data
    #[arg(long)]
    pub combine_phase: Option<bool>,

    /// BET fractional intensity (0.0-1.0)
    #[arg(long)]
    pub bet_fractional_intensity: Option<f64>,

    /// BET surface smoothness factor
    #[arg(long)]
    pub bet_smoothness: Option<f64>,

    /// BET gradient threshold (-1 to 1)
    #[arg(long)]
    pub bet_gradient_threshold: Option<f64>,

    /// BET surface evolution iterations
    #[arg(long)]
    pub bet_iterations: Option<usize>,

    /// BET icosphere subdivision level
    #[arg(long)]
    pub bet_subdivisions: Option<usize>,

    /// QSM reference method (mean or none)
    #[arg(long, value_enum)]
    pub qsm_reference: Option<QsmReferenceArg>,

    /// TGV alpha1 (first-order weight)
    #[arg(long)]
    pub tgv_alpha1: Option<f64>,

    /// TGV alpha0 (second-order weight)
    #[arg(long)]
    pub tgv_alpha0: Option<f64>,

    /// Mask erosion iterations
    #[arg(long, num_args = 1..)]
    pub mask_erosions: Option<Vec<usize>>,

    /// RTS delta parameter
    #[arg(long)]
    pub rts_delta: Option<f64>,

    /// RTS mu parameter
    #[arg(long)]
    pub rts_mu: Option<f64>,

    /// RTS tolerance
    #[arg(long)]
    pub rts_tol: Option<f64>,

    /// RTS rho (ADMM penalty)
    #[arg(long)]
    pub rts_rho: Option<f64>,

    /// RTS max iterations
    #[arg(long)]
    pub rts_max_iter: Option<usize>,

    /// RTS LSMR iterations
    #[arg(long)]
    pub rts_lsmr_iter: Option<usize>,

    /// TGV iterations
    #[arg(long)]
    pub tgv_iterations: Option<usize>,

    /// TGV erosions
    #[arg(long)]
    pub tgv_erosions: Option<usize>,

    /// TV lambda parameter
    #[arg(long)]
    pub tv_lambda: Option<f64>,

    /// TV rho (ADMM penalty)
    #[arg(long)]
    pub tv_rho: Option<f64>,

    /// TV tolerance
    #[arg(long)]
    pub tv_tol: Option<f64>,

    /// TV max iterations
    #[arg(long)]
    pub tv_max_iter: Option<usize>,

    /// TKD threshold
    #[arg(long)]
    pub tkd_threshold: Option<f64>,

    // Execution
    /// Number of parallel threads
    #[arg(long)]
    pub n_procs: Option<usize>,

    /// Also compute SWI
    #[arg(long)]
    pub do_swi: bool,

    /// Compute T2* relaxation map from multi-echo magnitude data
    #[arg(long)]
    pub do_t2starmap: bool,

    /// Compute R2* decay rate map from multi-echo magnitude data
    #[arg(long)]
    pub do_r2starmap: bool,

    /// Apply inhomogeneity correction to magnitude before masking
    #[arg(long)]
    pub inhomogeneity_correction: bool,

    /// Resample oblique acquisitions to axial if obliquity exceeds threshold (degrees, -1 to disable)
    #[arg(long)]
    pub obliquity_threshold: Option<f64>,

    /// Mask-building operation (repeatable, applied in order). Overrides --masking-algorithm.
    /// Format: input:magnitude, threshold:otsu, erode:2, dilate:1, close:1, fill-holes:1000, gaussian:4.0, bet:0.5
    #[arg(long = "mask-op", num_args = 1)]
    pub mask_ops: Option<Vec<String>>,

    /// Print processing plan without executing
    #[arg(long)]
    pub dry: bool,

    /// Enable debug logging
    #[arg(long)]
    pub debug: bool,

    /// Memory limit in GB for concurrent run scheduling (auto-detected if not specified)
    #[arg(long)]
    pub mem_limit_gb: Option<f64>,

    /// Disable memory-aware concurrency limiting
    #[arg(long)]
    pub no_mem_limit: bool,

    /// Force re-run, ignoring cached pipeline state
    #[arg(long)]
    pub force: bool,

    /// Remove intermediate files after pipeline completes (keep only final outputs)
    #[arg(long)]
    pub clean_intermediates: bool,
}

#[derive(Parser, Debug)]
pub struct InitArgs {
    /// Base preset for the configuration
    #[arg(long, value_enum, default_value = "gre")]
    pub preset: Preset,

    /// Output file path (prints to stdout if not specified)
    #[arg(short, long)]
    pub output: Option<PathBuf>,
}

#[derive(Parser, Debug)]
pub struct ValidateArgs {
    /// Input BIDS directory
    pub bids_dir: PathBuf,

    /// Check only these subjects
    #[arg(long, num_args = 1..)]
    pub subjects: Option<Vec<String>>,

    /// Check only these sessions
    #[arg(long, num_args = 1..)]
    pub sessions: Option<Vec<String>>,
}

#[derive(Parser, Debug)]
pub struct PresetsArgs {
    /// Show details for a specific preset
    pub name: Option<String>,
}

#[derive(Parser, Debug)]
pub struct SlurmArgs {
    /// Input BIDS directory
    pub bids_dir: PathBuf,

    /// Output derivatives directory
    pub output_dir: PathBuf,

    /// SLURM account name
    #[arg(long)]
    pub account: String,

    /// SLURM partition
    #[arg(long)]
    pub partition: Option<String>,

    /// Use a pipeline preset
    #[arg(long, value_enum)]
    pub preset: Option<Preset>,

    /// Pipeline configuration file (TOML)
    #[arg(long)]
    pub config: Option<PathBuf>,

    /// Wall time limit (e.g., 02:00:00)
    #[arg(long, default_value = "02:00:00")]
    pub time: String,

    /// Memory per job in GB
    #[arg(long, default_value_t = 32)]
    pub mem: usize,

    /// CPUs per task
    #[arg(long, default_value_t = 4)]
    pub cpus_per_task: usize,

    /// Auto-submit scripts via sbatch
    #[arg(long)]
    pub submit: bool,
}

// ─── Algorithm commands ───

#[derive(Parser, Debug)]
pub struct BetArgs {
    /// Input magnitude NIfTI file
    pub input: PathBuf,

    /// Output mask NIfTI file
    #[arg(short, long)]
    pub output: PathBuf,

    /// Fractional intensity threshold (0.0-1.0, smaller = larger brain)
    #[arg(long, default_value_t = 0.5)]
    pub fractional_intensity: f64,

    /// Surface smoothness factor
    #[arg(long, default_value_t = 1.0)]
    pub smoothness: f64,

    /// Gradient threshold (-1 to 1)
    #[arg(long, default_value_t = 0.0)]
    pub gradient_threshold: f64,

    /// Number of iterations
    #[arg(long, default_value_t = 1000)]
    pub iterations: usize,

    /// Icosphere subdivision level
    #[arg(long, default_value_t = 4)]
    pub subdivisions: usize,
}

#[derive(Parser, Debug)]
pub struct MaskArgs {
    /// Input NIfTI file
    pub input: PathBuf,

    /// Output mask NIfTI file
    #[arg(short, long)]
    pub output: PathBuf,

    /// Thresholding method
    #[arg(long, value_enum, default_value = "otsu")]
    pub method: ThresholdMethod,

    /// Manual threshold value (for method=value)
    #[arg(long)]
    pub threshold: Option<f64>,

    /// Number of mask erosion iterations
    #[arg(long, default_value_t = 0)]
    pub erosions: usize,
}

#[derive(Parser, Debug)]
pub struct UnwrapArgs {
    /// Input wrapped phase NIfTI file
    pub input: PathBuf,

    /// Binary mask NIfTI file
    #[arg(short, long)]
    pub mask: PathBuf,

    /// Output unwrapped phase NIfTI file
    #[arg(short, long)]
    pub output: PathBuf,

    /// Unwrapping algorithm
    #[arg(long, value_enum, default_value = "laplacian")]
    pub algorithm: UnwrapAlgorithmArg,

    /// Magnitude image (recommended for ROMEO)
    #[arg(long)]
    pub magnitude: Option<PathBuf>,
}

#[derive(Parser, Debug)]
pub struct BgremoveArgs {
    /// Input total field NIfTI file
    pub input: PathBuf,

    /// Binary mask NIfTI file
    #[arg(short, long)]
    pub mask: PathBuf,

    /// Output local field NIfTI file
    #[arg(short, long)]
    pub output: PathBuf,

    /// Background removal algorithm
    #[arg(long, value_enum, default_value = "vsharp")]
    pub algorithm: BfAlgorithmArg,

    /// B0 direction (3 values)
    #[arg(long, num_args = 3, default_values_t = [0.0, 0.0, 1.0])]
    pub b0_direction: Vec<f64>,

    /// Output eroded mask (for algorithms that erode)
    #[arg(long)]
    pub output_mask: Option<PathBuf>,
}

#[derive(Parser, Debug)]
pub struct InvertArgs {
    /// Input local field NIfTI file
    pub input: PathBuf,

    /// Binary mask NIfTI file
    #[arg(short, long)]
    pub mask: PathBuf,

    /// Output susceptibility map NIfTI file
    #[arg(short, long)]
    pub output: PathBuf,

    /// Dipole inversion algorithm
    #[arg(long, value_enum, default_value = "rts")]
    pub algorithm: QsmAlgorithmArg,

    /// B0 direction (3 values)
    #[arg(long, num_args = 3, default_values_t = [0.0, 0.0, 1.0])]
    pub b0_direction: Vec<f64>,

    // RTS parameters
    #[arg(long, default_value_t = 0.15)]
    pub rts_delta: f64,
    #[arg(long, default_value_t = 1e5)]
    pub rts_mu: f64,
    #[arg(long, default_value_t = 0.01)]
    pub rts_tol: f64,
    #[arg(long, default_value_t = 10.0)]
    pub rts_rho: f64,
    #[arg(long, default_value_t = 20)]
    pub rts_max_iter: usize,
    #[arg(long, default_value_t = 4)]
    pub rts_lsmr_iter: usize,

    // TV parameters
    #[arg(long, default_value_t = 0.0002)]
    pub tv_lambda: f64,
    #[arg(long, default_value_t = 0.02)]
    pub tv_rho: f64,
    #[arg(long, default_value_t = 0.001)]
    pub tv_tol: f64,
    #[arg(long, default_value_t = 250)]
    pub tv_max_iter: usize,

    // TKD parameters
    #[arg(long, default_value_t = 0.15)]
    pub tkd_threshold: f64,

    // TGV parameters
    #[arg(long, default_value_t = 1000)]
    pub tgv_iterations: usize,
    #[arg(long, default_value_t = 3)]
    pub tgv_erosions: usize,

    /// B0 field strength in Tesla (required for TGV)
    #[arg(long)]
    pub field_strength: Option<f64>,

    /// Echo time in seconds (required for TGV)
    #[arg(long)]
    pub echo_time: Option<f64>,
}

#[derive(Parser, Debug)]
pub struct SwiArgs {
    /// Input phase NIfTI file
    pub phase: PathBuf,

    /// Input magnitude NIfTI file
    pub magnitude: PathBuf,

    /// Binary mask NIfTI file
    #[arg(short, long)]
    pub mask: PathBuf,

    /// Output SWI NIfTI file
    #[arg(short, long)]
    pub output: PathBuf,

    /// Also compute minimum intensity projection
    #[arg(long)]
    pub mip: bool,

    /// Output path for MIP
    #[arg(long)]
    pub mip_output: Option<PathBuf>,
}

#[derive(Parser, Debug)]
pub struct R2starArgs {
    /// Input multi-echo magnitude NIfTI files (3+ echoes required)
    #[arg(required = true, num_args = 3..)]
    pub inputs: Vec<PathBuf>,

    /// Binary mask NIfTI file
    #[arg(short, long)]
    pub mask: PathBuf,

    /// Output R2* map NIfTI file
    #[arg(short, long)]
    pub output: PathBuf,

    /// Echo times in seconds (must match number of inputs)
    #[arg(long, required = true, num_args = 3..)]
    pub echo_times: Vec<f64>,
}

#[derive(Parser, Debug)]
pub struct T2starArgs {
    /// Input multi-echo magnitude NIfTI files (3+ echoes required)
    #[arg(required = true, num_args = 3..)]
    pub inputs: Vec<PathBuf>,

    /// Binary mask NIfTI file
    #[arg(short, long)]
    pub mask: PathBuf,

    /// Output T2* map NIfTI file
    #[arg(short, long)]
    pub output: PathBuf,

    /// Echo times in seconds (must match number of inputs)
    #[arg(long, required = true, num_args = 3..)]
    pub echo_times: Vec<f64>,
}

#[derive(Parser, Debug)]
pub struct HomogeneityArgs {
    /// Input magnitude NIfTI file
    pub input: PathBuf,

    /// Output corrected magnitude NIfTI file
    #[arg(short, long)]
    pub output: PathBuf,

    /// Smoothing sigma in mm (default: 7.0)
    #[arg(long, default_value_t = 7.0)]
    pub sigma: f64,

    /// Number of box filter passes for Gaussian approximation (default: 3)
    #[arg(long, default_value_t = 3)]
    pub nbox: usize,
}

#[derive(Parser, Debug)]
pub struct DilateArgs {
    /// Input binary mask NIfTI file
    pub input: PathBuf,

    /// Output dilated mask NIfTI file
    #[arg(short, long)]
    pub output: PathBuf,

    /// Number of dilation iterations
    #[arg(long, default_value_t = 1)]
    pub iterations: usize,
}

#[derive(Parser, Debug)]
pub struct CloseArgs {
    /// Input binary mask NIfTI file
    pub input: PathBuf,

    /// Output closed mask NIfTI file
    #[arg(short, long)]
    pub output: PathBuf,

    /// Closing radius
    #[arg(long, default_value_t = 1)]
    pub radius: usize,
}

#[derive(Parser, Debug)]
pub struct FillHolesArgs {
    /// Input binary mask NIfTI file
    pub input: PathBuf,

    /// Output filled mask NIfTI file
    #[arg(short, long)]
    pub output: PathBuf,

    /// Maximum hole size in voxels
    #[arg(long, default_value_t = 1000)]
    pub max_size: usize,
}

#[derive(Parser, Debug)]
pub struct SmoothMaskArgs {
    /// Input binary mask NIfTI file
    pub input: PathBuf,

    /// Output smoothed mask NIfTI file
    #[arg(short, long)]
    pub output: PathBuf,

    /// Gaussian sigma in mm
    #[arg(long, default_value_t = 4.0)]
    pub sigma: f64,
}

#[derive(Parser, Debug)]
pub struct ResampleArgs {
    /// Input NIfTI file
    pub input: PathBuf,

    /// Output resampled NIfTI file
    #[arg(short, long)]
    pub output: PathBuf,
}

#[derive(Parser, Debug)]
pub struct QualityMapArgs {
    /// Input phase NIfTI file (first echo)
    pub phase: PathBuf,

    /// Output quality map NIfTI file
    #[arg(short, long)]
    pub output: PathBuf,

    /// Magnitude image (improves quality estimation)
    #[arg(long)]
    pub magnitude: Option<PathBuf>,

    /// Second echo phase image (improves quality estimation)
    #[arg(long)]
    pub phase2: Option<PathBuf>,

    /// Echo time of first phase in seconds
    #[arg(long, default_value_t = 0.02)]
    pub te1: f64,

    /// Echo time of second phase in seconds (if --phase2 provided)
    #[arg(long, default_value_t = 0.04)]
    pub te2: f64,
}

// ─── Shared enums ───

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq)]
pub enum Preset {
    Gre,
    Epi,
    Bet,
    Fast,
    Body,
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq)]
pub enum QsmAlgorithmArg {
    Rts,
    Tv,
    Tkd,
    Tgv,
    Tikhonov,
    Nltv,
    Medi,
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq)]
pub enum UnwrapAlgorithmArg {
    Romeo,
    Laplacian,
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq)]
pub enum BfAlgorithmArg {
    Vsharp,
    Pdf,
    Lbv,
    Ismv,
    Sharp,
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq)]
pub enum MaskAlgorithmArg {
    Bet,
    Threshold,
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq)]
pub enum MaskInputArg {
    MagnitudeFirst,
    Magnitude,
    MagnitudeLast,
    PhaseQuality,
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq)]
pub enum QsmReferenceArg {
    Mean,
    None,
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq)]
pub enum ThresholdMethod {
    Otsu,
    Value,
}
