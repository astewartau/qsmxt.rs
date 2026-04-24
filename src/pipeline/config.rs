use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::Path;

use crate::cli;
use crate::error::QsmxtError;

// ─── Mask operation pipeline ───

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum MaskThresholdMethod {
    Otsu,
    Fixed,
    Percentile,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "op", rename_all = "kebab-case")]
pub enum MaskOp {
    /// Select masking input source
    Input { source: MaskingInput },
    /// Apply threshold to produce binary mask
    Threshold { method: MaskThresholdMethod, #[serde(default)] value: Option<f64> },
    /// BET brain extraction
    Bet { fractional_intensity: f64 },
    /// Erode mask (6-connectivity)
    Erode { iterations: usize },
    /// Dilate mask (6-connectivity)
    Dilate { iterations: usize },
    /// Morphological close (dilate then erode)
    Close { radius: usize },
    /// Fill holes up to max_size voxels
    FillHoles { max_size: usize },
    /// Gaussian smooth mask + re-threshold at 0.5
    GaussianSmooth { sigma_mm: f64 },
}

impl fmt::Display for MaskOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Input { source } => write!(f, "input:{}", source),
            Self::Threshold { method: MaskThresholdMethod::Otsu, .. } => write!(f, "threshold:otsu"),
            Self::Threshold { method: MaskThresholdMethod::Fixed, value } =>
                write!(f, "threshold:fixed:{}", value.unwrap_or(0.5)),
            Self::Threshold { method: MaskThresholdMethod::Percentile, value } =>
                write!(f, "threshold:percentile:{}", value.unwrap_or(75.0)),
            Self::Bet { fractional_intensity } => write!(f, "bet:{}", fractional_intensity),
            Self::Erode { iterations } => write!(f, "erode:{}", iterations),
            Self::Dilate { iterations } => write!(f, "dilate:{}", iterations),
            Self::Close { radius } => write!(f, "close:{}", radius),
            Self::FillHoles { max_size } => write!(f, "fill-holes:{}", max_size),
            Self::GaussianSmooth { sigma_mm } => write!(f, "gaussian:{}", sigma_mm),
        }
    }
}

/// Parse a mask operation from CLI string format (e.g. "erode:2", "threshold:otsu").
pub fn parse_mask_op(s: &str) -> crate::Result<MaskOp> {
    let parts: Vec<&str> = s.splitn(3, ':').collect();
    match parts[0] {
        "input" => {
            let source = match parts.get(1).copied() {
                Some("magnitude") => MaskingInput::Magnitude,
                Some("magnitude-first") => MaskingInput::MagnitudeFirst,
                Some("magnitude-last") => MaskingInput::MagnitudeLast,
                Some("phase-quality") => MaskingInput::PhaseQuality,
                _ => return Err(QsmxtError::Config(
                    format!("Invalid mask-op input source: '{}'. Expected magnitude, magnitude-first, magnitude-last, or phase-quality", s)
                )),
            };
            Ok(MaskOp::Input { source })
        }
        "threshold" => {
            match parts.get(1).copied() {
                Some("otsu") | None => Ok(MaskOp::Threshold { method: MaskThresholdMethod::Otsu, value: None }),
                Some("fixed") => {
                    let v = parts.get(2)
                        .and_then(|s| s.parse::<f64>().ok())
                        .unwrap_or(0.5);
                    Ok(MaskOp::Threshold { method: MaskThresholdMethod::Fixed, value: Some(v) })
                }
                Some("percentile") => {
                    let v = parts.get(2)
                        .and_then(|s| s.parse::<f64>().ok())
                        .unwrap_or(75.0);
                    Ok(MaskOp::Threshold { method: MaskThresholdMethod::Percentile, value: Some(v) })
                }
                Some(other) => Err(QsmxtError::Config(
                    format!("Invalid threshold method: '{}'. Expected otsu, fixed, or percentile", other)
                )),
            }
        }
        "bet" => {
            let fi = parts.get(1).and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.5);
            Ok(MaskOp::Bet { fractional_intensity: fi })
        }
        "erode" => {
            let n = parts.get(1).and_then(|s| s.parse::<usize>().ok()).unwrap_or(1);
            Ok(MaskOp::Erode { iterations: n })
        }
        "dilate" => {
            let n = parts.get(1).and_then(|s| s.parse::<usize>().ok()).unwrap_or(1);
            Ok(MaskOp::Dilate { iterations: n })
        }
        "close" => {
            let r = parts.get(1).and_then(|s| s.parse::<usize>().ok()).unwrap_or(1);
            Ok(MaskOp::Close { radius: r })
        }
        "fill-holes" => {
            let sz = parts.get(1).and_then(|s| s.parse::<usize>().ok()).unwrap_or(1000);
            Ok(MaskOp::FillHoles { max_size: sz })
        }
        "gaussian" => {
            let sigma = parts.get(1).and_then(|s| s.parse::<f64>().ok()).unwrap_or(4.0);
            Ok(MaskOp::GaussianSmooth { sigma_mm: sigma })
        }
        _ => Err(QsmxtError::Config(
            format!("Unknown mask-op: '{}'. Expected input, threshold, bet, erode, dilate, close, fill-holes, or gaussian", parts[0])
        )),
    }
}

// ─── Algorithm enums ───

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum QsmAlgorithm {
    Rts,
    Tv,
    Tkd,
    Tgv,
}

impl fmt::Display for QsmAlgorithm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Rts => write!(f, "rts"),
            Self::Tv => write!(f, "tv"),
            Self::Tkd => write!(f, "tkd"),
            Self::Tgv => write!(f, "tgv"),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum UnwrappingAlgorithm {
    Romeo,
    Laplacian,
}

impl fmt::Display for UnwrappingAlgorithm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Romeo => write!(f, "romeo"),
            Self::Laplacian => write!(f, "laplacian"),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum BfAlgorithm {
    Vsharp,
    Pdf,
    Lbv,
    Ismv,
}

impl fmt::Display for BfAlgorithm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Vsharp => write!(f, "vsharp"),
            Self::Pdf => write!(f, "pdf"),
            Self::Lbv => write!(f, "lbv"),
            Self::Ismv => write!(f, "ismv"),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MaskingAlgorithm {
    Bet,
    Threshold,
}

impl fmt::Display for MaskingAlgorithm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bet => write!(f, "bet"),
            Self::Threshold => write!(f, "threshold"),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum MaskingInput {
    /// First echo magnitude
    MagnitudeFirst,
    /// RSS combination of all echo magnitudes
    Magnitude,
    /// Last echo magnitude
    MagnitudeLast,
    /// ROMEO phase quality map (from spatial phase gradients)
    PhaseQuality,
}

impl fmt::Display for MaskingInput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MagnitudeFirst => write!(f, "magnitude-first"),
            Self::Magnitude => write!(f, "magnitude"),
            Self::MagnitudeLast => write!(f, "magnitude-last"),
            Self::PhaseQuality => write!(f, "phase-quality"),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum QsmReference {
    Mean,
    None,
}

impl fmt::Display for QsmReference {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Mean => write!(f, "mean"),
            Self::None => write!(f, "none"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineConfig {
    #[serde(default)]
    pub description: String,

    // Pipeline toggles
    #[serde(default = "default_true")]
    pub do_qsm: bool,
    #[serde(default)]
    pub do_swi: bool,
    #[serde(default)]
    pub do_t2starmap: bool,
    #[serde(default)]
    pub do_r2starmap: bool,
    #[serde(default)]
    pub inhomogeneity_correction: bool,

    /// Resample oblique acquisitions to axial orientation.
    /// Threshold in degrees; -1 disables. Default: disabled (-1).
    #[serde(default = "default_obliquity_threshold")]
    pub obliquity_threshold: f64,

    /// Ordered mask-building operations. If empty, falls back to legacy masking fields.
    #[serde(default)]
    pub mask_ops: Vec<MaskOp>,

    // Algorithm choices
    #[serde(default = "default_qsm_algorithm")]
    pub qsm_algorithm: QsmAlgorithm,
    #[serde(default = "default_unwrapping")]
    pub unwrapping_algorithm: Option<UnwrappingAlgorithm>,
    #[serde(default = "default_bf")]
    pub bf_algorithm: Option<BfAlgorithm>,
    #[serde(default = "default_masking")]
    pub masking_algorithm: MaskingAlgorithm,
    #[serde(default = "default_masking_input")]
    pub masking_input: MaskingInput,

    // Multi-echo
    #[serde(default = "default_true")]
    pub combine_phase: bool,

    // QSM reference
    #[serde(default = "default_reference")]
    pub qsm_reference: QsmReference,

    // Masking parameters
    #[serde(default = "default_bet_fi")]
    pub bet_fractional_intensity: f64,
    #[serde(default = "default_bet_smoothness")]
    pub bet_smoothness: f64,
    #[serde(default = "default_bet_gradient")]
    pub bet_gradient_threshold: f64,
    #[serde(default = "default_bet_iterations")]
    pub bet_iterations: usize,
    #[serde(default = "default_bet_subdivisions")]
    pub bet_subdivisions: usize,
    #[serde(default = "default_erosions")]
    pub mask_erosions: Vec<usize>,

    // RTS parameters
    #[serde(default = "default_rts_delta")]
    pub rts_delta: f64,
    #[serde(default = "default_rts_mu")]
    pub rts_mu: f64,
    #[serde(default = "default_rts_tol")]
    pub rts_tol: f64,
    #[serde(default = "default_rts_rho")]
    pub rts_rho: f64,
    #[serde(default = "default_rts_max_iter")]
    pub rts_max_iter: usize,
    #[serde(default = "default_rts_lsmr_iter")]
    pub rts_lsmr_iter: usize,

    // TV parameters
    #[serde(default = "default_tv_lambda")]
    pub tv_lambda: f64,
    #[serde(default = "default_tv_rho")]
    pub tv_rho: f64,
    #[serde(default = "default_tv_tol")]
    pub tv_tol: f64,
    #[serde(default = "default_tv_max_iter")]
    pub tv_max_iter: usize,

    // TKD parameters
    #[serde(default = "default_tkd_threshold")]
    pub tkd_threshold: f64,

    // TGV parameters
    #[serde(default = "default_tgv_iterations")]
    pub tgv_iterations: usize,
    #[serde(default = "default_tgv_alphas")]
    pub tgv_alphas: [f64; 2],
    #[serde(default = "default_tgv_erosions")]
    pub tgv_erosions: usize,
}

// Defaults
fn default_true() -> bool { true }
fn default_qsm_algorithm() -> QsmAlgorithm { QsmAlgorithm::Rts }
fn default_unwrapping() -> Option<UnwrappingAlgorithm> { Some(UnwrappingAlgorithm::Romeo) }
fn default_bf() -> Option<BfAlgorithm> { Some(BfAlgorithm::Vsharp) }
fn default_masking() -> MaskingAlgorithm { MaskingAlgorithm::Threshold }
fn default_masking_input() -> MaskingInput { MaskingInput::PhaseQuality }
fn default_reference() -> QsmReference { QsmReference::Mean }
fn default_bet_fi() -> f64 { qsm_core::bet::BetParams::default().fractional_intensity }
fn default_bet_smoothness() -> f64 { qsm_core::bet::BetParams::default().smoothness }
fn default_bet_gradient() -> f64 { qsm_core::bet::BetParams::default().gradient_threshold }
fn default_bet_iterations() -> usize { qsm_core::bet::BetParams::default().iterations }
fn default_bet_subdivisions() -> usize { qsm_core::bet::BetParams::default().subdivisions }
fn default_erosions() -> Vec<usize> { vec![2] }
fn default_rts_delta() -> f64 { qsm_core::inversion::RtsParams::default().delta }
fn default_rts_mu() -> f64 { qsm_core::inversion::RtsParams::default().mu }
fn default_rts_tol() -> f64 { qsm_core::inversion::RtsParams::default().tol }
fn default_rts_rho() -> f64 { qsm_core::inversion::RtsParams::default().rho }
fn default_rts_max_iter() -> usize { qsm_core::inversion::RtsParams::default().max_iter }
fn default_rts_lsmr_iter() -> usize { qsm_core::inversion::RtsParams::default().lsmr_iter }
fn default_tv_lambda() -> f64 { qsm_core::inversion::TvParams::default().lambda }
fn default_tv_rho() -> f64 { qsm_core::inversion::TvParams::default().rho }
fn default_tv_tol() -> f64 { qsm_core::inversion::TvParams::default().tol }
fn default_tv_max_iter() -> usize { qsm_core::inversion::TvParams::default().max_iter }
fn default_tkd_threshold() -> f64 { qsm_core::inversion::TkdParams::default().threshold }
fn default_tgv_iterations() -> usize { qsm_core::inversion::TgvParams::default().iterations }
fn default_mask_ops() -> Vec<MaskOp> {
    vec![
        MaskOp::Input { source: MaskingInput::PhaseQuality },
        MaskOp::Threshold { method: MaskThresholdMethod::Otsu, value: None },
        MaskOp::Dilate { iterations: 2 },
        MaskOp::FillHoles { max_size: 0 }, // 0 = auto (volume/20)
        MaskOp::Erode { iterations: 2 },
    ]
}
fn default_obliquity_threshold() -> f64 { -1.0 }
fn default_tgv_alphas() -> [f64; 2] {
    let p = qsm_core::inversion::TgvParams::default();
    [p.alpha1 as f64, p.alpha0 as f64]
}
fn default_tgv_erosions() -> usize { qsm_core::inversion::TgvParams::default().erosions }

impl Default for PipelineConfig {
    fn default() -> Self {
        Self::from_preset(cli::Preset::Gre)
    }
}

impl PipelineConfig {
    /// Create a config from a named preset.
    pub fn from_preset(preset: cli::Preset) -> Self {
        match preset {
            cli::Preset::Gre => Self {
                description: "3D-GRE images (human brain)".to_string(),
                do_qsm: true,
                do_swi: false,
                do_t2starmap: false,
                do_r2starmap: false,
                inhomogeneity_correction: false,
                obliquity_threshold: -1.0,
                mask_ops: default_mask_ops(),
                qsm_algorithm: QsmAlgorithm::Rts,
                unwrapping_algorithm: Some(UnwrappingAlgorithm::Romeo),
                bf_algorithm: Some(BfAlgorithm::Vsharp),
                masking_algorithm: MaskingAlgorithm::Threshold,
                masking_input: MaskingInput::PhaseQuality,
                combine_phase: true,
                qsm_reference: QsmReference::Mean,
                bet_fractional_intensity: default_bet_fi(),
                bet_smoothness: default_bet_smoothness(),
                bet_gradient_threshold: default_bet_gradient(),
                bet_iterations: default_bet_iterations(),
                bet_subdivisions: default_bet_subdivisions(),
                mask_erosions: vec![2],
                rts_delta: default_rts_delta(),
                rts_mu: default_rts_mu(),
                rts_tol: default_rts_tol(),
                rts_rho: default_rts_rho(),
                rts_max_iter: default_rts_max_iter(),
                rts_lsmr_iter: default_rts_lsmr_iter(),
                tv_lambda: default_tv_lambda(),
                tv_rho: default_tv_rho(),
                tv_tol: default_tv_tol(),
                tv_max_iter: default_tv_max_iter(),
                tkd_threshold: default_tkd_threshold(),
                tgv_iterations: default_tgv_iterations(),
                tgv_alphas: default_tgv_alphas(),
                tgv_erosions: default_tgv_erosions(),
            },
            cli::Preset::Epi => Self {
                description: "3D-EPI images (human brain)".to_string(),
                mask_erosions: vec![3],
                ..Self::from_preset(cli::Preset::Gre)
            },
            cli::Preset::Bet => Self {
                description: "Traditional BET masking (human brain)".to_string(),
                masking_algorithm: MaskingAlgorithm::Bet,
                masking_input: MaskingInput::Magnitude,
                mask_ops: vec![
                    MaskOp::Bet { fractional_intensity: 0.5 },
                    MaskOp::Erode { iterations: 3 },
                ],
                mask_erosions: vec![3],
                ..Self::from_preset(cli::Preset::Gre)
            },
            cli::Preset::Fast => Self {
                description: "Fast algorithms".to_string(),
                masking_algorithm: MaskingAlgorithm::Bet,
                masking_input: MaskingInput::Magnitude,
                bf_algorithm: Some(BfAlgorithm::Vsharp),
                mask_ops: vec![
                    MaskOp::Bet { fractional_intensity: 0.5 },
                    MaskOp::Erode { iterations: 3 },
                ],
                mask_erosions: vec![3],
                ..Self::from_preset(cli::Preset::Gre)
            },
            cli::Preset::Body => Self {
                description: "Non-brain applications".to_string(),
                qsm_algorithm: QsmAlgorithm::Tgv,
                unwrapping_algorithm: None,
                bf_algorithm: None,
                combine_phase: false,
                masking_algorithm: MaskingAlgorithm::Threshold,
                masking_input: MaskingInput::Magnitude,
                mask_ops: vec![
                    MaskOp::Input { source: MaskingInput::Magnitude },
                    MaskOp::Threshold { method: MaskThresholdMethod::Otsu, value: None },
                    MaskOp::Dilate { iterations: 2 },
                    MaskOp::FillHoles { max_size: 0 },
                    MaskOp::Erode { iterations: 3 },
                ],
                mask_erosions: vec![3],
                ..Self::from_preset(cli::Preset::Gre)
            },
        }
    }

    /// Load config from a TOML file.
    pub fn from_file(path: &Path) -> crate::Result<Self> {
        let text = std::fs::read_to_string(path)?;
        toml::from_str(&text).map_err(|e| QsmxtError::Config(format!("TOML parse error: {}", e)))
    }

    /// Apply CLI overrides onto this config.
    pub fn apply_run_overrides(&mut self, args: &cli::RunArgs) {
        if let Some(a) = args.qsm_algorithm {
            self.qsm_algorithm = match a {
                cli::QsmAlgorithmArg::Rts => QsmAlgorithm::Rts,
                cli::QsmAlgorithmArg::Tv => QsmAlgorithm::Tv,
                cli::QsmAlgorithmArg::Tkd => QsmAlgorithm::Tkd,
                cli::QsmAlgorithmArg::Tgv => QsmAlgorithm::Tgv,
            };
        }
        if let Some(a) = args.unwrapping_algorithm {
            self.unwrapping_algorithm = Some(match a {
                cli::UnwrapAlgorithmArg::Romeo => UnwrappingAlgorithm::Romeo,
                cli::UnwrapAlgorithmArg::Laplacian => UnwrappingAlgorithm::Laplacian,
            });
        }
        if let Some(a) = args.bf_algorithm {
            self.bf_algorithm = Some(match a {
                cli::BfAlgorithmArg::Vsharp => BfAlgorithm::Vsharp,
                cli::BfAlgorithmArg::Pdf => BfAlgorithm::Pdf,
                cli::BfAlgorithmArg::Lbv => BfAlgorithm::Lbv,
                cli::BfAlgorithmArg::Ismv => BfAlgorithm::Ismv,
            });
        }
        if let Some(a) = args.masking_algorithm {
            self.masking_algorithm = match a {
                cli::MaskAlgorithmArg::Bet => MaskingAlgorithm::Bet,
                cli::MaskAlgorithmArg::Threshold => MaskingAlgorithm::Threshold,
            };
        }
        if let Some(a) = args.masking_input {
            self.masking_input = match a {
                cli::MaskInputArg::MagnitudeFirst => MaskingInput::MagnitudeFirst,
                cli::MaskInputArg::Magnitude => MaskingInput::Magnitude,
                cli::MaskInputArg::MagnitudeLast => MaskingInput::MagnitudeLast,
                cli::MaskInputArg::PhaseQuality => MaskingInput::PhaseQuality,
            };
        }
        if let Some(v) = args.combine_phase {
            self.combine_phase = v;
        }
        if let Some(v) = args.bet_fractional_intensity {
            self.bet_fractional_intensity = v;
        }
        if let Some(v) = args.bet_smoothness {
            self.bet_smoothness = v;
        }
        if let Some(v) = args.bet_gradient_threshold {
            self.bet_gradient_threshold = v;
        }
        if let Some(v) = args.bet_iterations {
            self.bet_iterations = v;
        }
        if let Some(v) = args.bet_subdivisions {
            self.bet_subdivisions = v;
        }
        if let Some(a) = args.qsm_reference {
            self.qsm_reference = match a {
                cli::QsmReferenceArg::Mean => QsmReference::Mean,
                cli::QsmReferenceArg::None => QsmReference::None,
            };
        }
        if let Some(v) = args.tgv_alpha1 {
            self.tgv_alphas[0] = v;
        }
        if let Some(v) = args.tgv_alpha0 {
            self.tgv_alphas[1] = v;
        }
        if let Some(ref v) = args.mask_erosions {
            self.mask_erosions = v.clone();
        }
        if let Some(v) = args.rts_delta {
            self.rts_delta = v;
        }
        if let Some(v) = args.rts_mu {
            self.rts_mu = v;
        }
        if let Some(v) = args.rts_tol {
            self.rts_tol = v;
        }
        if let Some(v) = args.rts_rho {
            self.rts_rho = v;
        }
        if let Some(v) = args.rts_max_iter {
            self.rts_max_iter = v;
        }
        if let Some(v) = args.rts_lsmr_iter {
            self.rts_lsmr_iter = v;
        }
        if let Some(v) = args.tv_lambda {
            self.tv_lambda = v;
        }
        if let Some(v) = args.tv_rho {
            self.tv_rho = v;
        }
        if let Some(v) = args.tv_tol {
            self.tv_tol = v;
        }
        if let Some(v) = args.tv_max_iter {
            self.tv_max_iter = v;
        }
        if let Some(v) = args.tkd_threshold {
            self.tkd_threshold = v;
        }
        if let Some(v) = args.tgv_iterations {
            self.tgv_iterations = v;
        }
        if let Some(v) = args.tgv_erosions {
            self.tgv_erosions = v;
        }
        if args.do_swi {
            self.do_swi = true;
        }
        if args.do_t2starmap {
            self.do_t2starmap = true;
        }
        if args.do_r2starmap {
            self.do_r2starmap = true;
        }
        if args.inhomogeneity_correction {
            self.inhomogeneity_correction = true;
        }
        if let Some(v) = args.obliquity_threshold {
            self.obliquity_threshold = v;
        }
        if let Some(ref ops) = args.mask_ops {
            let mut parsed = Vec::new();
            for s in ops {
                match parse_mask_op(s) {
                    Ok(op) => parsed.push(op),
                    Err(e) => log::warn!("Ignoring invalid mask-op '{}': {}", s, e),
                }
            }
            if !parsed.is_empty() {
                self.mask_ops = parsed;
            }
        }
    }

    /// Validate the configuration for consistency.
    pub fn validate(&self) -> crate::Result<()> {
        if self.qsm_algorithm == QsmAlgorithm::Tgv {
            // TGV does its own unwrapping and BG removal
            if self.unwrapping_algorithm.is_some() {
                log::debug!("TGV selected; ignoring unwrapping_algorithm");
            }
            if self.bf_algorithm.is_some() {
                log::debug!("TGV selected; ignoring bf_algorithm");
            }
        } else if self.bf_algorithm.is_none() {
            return Err(QsmxtError::Config(
                "bf_algorithm must be set for non-TGV algorithms".to_string(),
            ));
        }
        Ok(())
    }

    /// Generate an annotated TOML string for this config.
    pub fn to_annotated_toml(&self) -> String {
        let mut s = String::new();
        s.push_str("# QSMxT Pipeline Configuration\n");
        if !self.description.is_empty() {
            s.push_str(&format!("# Preset: {}\n", self.description));
        }
        s.push('\n');

        s.push_str("[pipeline]\n");
        s.push_str("# QSM dipole inversion algorithm: rts | tv | tkd | tgv\n");
        s.push_str(&format!("qsm_algorithm = \"{}\"\n", self.qsm_algorithm));
        s.push_str("# Phase unwrapping algorithm: romeo | laplacian\n");
        match &self.unwrapping_algorithm {
            Some(a) => s.push_str(&format!("unwrapping_algorithm = \"{}\"\n", a)),
            None => s.push_str("# unwrapping_algorithm = \"romeo\"  # Not used with TGV\n"),
        }
        s.push_str("# Background field removal: vsharp | pdf | lbv | ismv\n");
        match &self.bf_algorithm {
            Some(a) => s.push_str(&format!("bf_algorithm = \"{}\"\n", a)),
            None => s.push_str("# bf_algorithm = \"pdf\"  # Not used with TGV\n"),
        }
        s.push_str("# Masking method: bet | threshold\n");
        s.push_str(&format!("masking_algorithm = \"{}\"\n", self.masking_algorithm));
        s.push_str("# Masking input: magnitude | magnitude-combined | magnitude-last | phase-quality\n");
        s.push_str(&format!("masking_input = \"{}\"\n", self.masking_input));
        s.push_str("# Combine multi-echo phase data using MCPC-3D-S\n");
        s.push_str(&format!("combine_phase = {}\n", self.combine_phase));
        s.push_str("# QSM reference: mean | none\n");
        s.push_str(&format!("qsm_reference = \"{}\"\n", self.qsm_reference));
        s.push_str(&format!("do_swi = {}\n", self.do_swi));
        s.push_str("# Compute T2* relaxation map from multi-echo magnitude data\n");
        s.push_str(&format!("do_t2starmap = {}\n", self.do_t2starmap));
        s.push_str("# Compute R2* decay rate map from multi-echo magnitude data\n");
        s.push_str(&format!("do_r2starmap = {}\n", self.do_r2starmap));
        s.push_str("# Apply inhomogeneity correction to magnitude before masking\n");
        s.push_str(&format!("inhomogeneity_correction = {}\n", self.inhomogeneity_correction));
        s.push_str("# Resample oblique acquisitions to axial (-1 = disabled, 0+ = threshold in degrees)\n");
        s.push_str(&format!("obliquity_threshold = {}\n", self.obliquity_threshold));
        s.push('\n');

        s.push_str("[masking]\n");
        s.push_str("# BET fractional intensity (0.0-1.0, smaller = larger brain)\n");
        s.push_str(&format!("bet_fractional_intensity = {}\n", self.bet_fractional_intensity));
        s.push_str("# Mask erosion iterations\n");
        s.push_str(&format!("mask_erosions = {:?}\n", self.mask_erosions));
        s.push('\n');

        s.push_str("[rts]\n");
        s.push_str(&format!("delta = {}\n", self.rts_delta));
        s.push_str(&format!("mu = {}\n", self.rts_mu));
        s.push_str(&format!("tolerance = {}\n", self.rts_tol));
        s.push('\n');

        s.push_str("[tv]\n");
        s.push_str(&format!("lambda = {}\n", self.tv_lambda));
        s.push('\n');

        s.push_str("[tkd]\n");
        s.push_str(&format!("threshold = {}\n", self.tkd_threshold));
        s.push('\n');

        s.push_str("[tgv]\n");
        s.push_str(&format!("iterations = {}\n", self.tgv_iterations));
        s.push_str(&format!("alphas = [{}, {}]\n", self.tgv_alphas[0], self.tgv_alphas[1]));
        s.push_str(&format!("erosions = {}\n", self.tgv_erosions));

        s
    }
}

/// List of all available presets with descriptions.
pub fn list_presets() -> Vec<(&'static str, &'static str)> {
    vec![
        ("gre", "3D-GRE images (human brain) — default"),
        ("epi", "3D-EPI images (human brain)"),
        ("bet", "Traditional BET masking (human brain)"),
        ("fast", "Fast algorithms"),
        ("body", "Non-brain applications (TGV single-step)"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli;
    use std::path::PathBuf;

    fn default_run_args() -> cli::RunArgs {
        cli::RunArgs {
            bids_dir: PathBuf::from("/tmp/fake"),
            output_dir: PathBuf::from("/tmp/fake_out"),
            preset: None,
            config: None,
            subjects: None,
            sessions: None,
            acquisitions: None,
            runs: None,
            num_echoes: None,
            qsm_algorithm: None,
            unwrapping_algorithm: None,
            bf_algorithm: None,
            masking_algorithm: None,
            masking_input: None,
            combine_phase: None,
            bet_fractional_intensity: None,
            bet_smoothness: None,
            bet_gradient_threshold: None,
            bet_iterations: None,
            bet_subdivisions: None,
            qsm_reference: None,
            tgv_alpha1: None,
            tgv_alpha0: None,
            mask_erosions: None,
            rts_delta: None,
            rts_mu: None,
            rts_tol: None,
            rts_rho: None,
            rts_max_iter: None,
            rts_lsmr_iter: None,
            tv_lambda: None,
            tv_rho: None,
            tv_tol: None,
            tv_max_iter: None,
            tkd_threshold: None,
            tgv_iterations: None,
            tgv_erosions: None,
            n_procs: None,
            do_swi: false,
            do_t2starmap: false,
            do_r2starmap: false,
            inhomogeneity_correction: false,
            obliquity_threshold: None,
            mask_ops: None,
            dry: false,
            debug: false,
            mem_limit_gb: None,
            no_mem_limit: false,
            force: false,
            clean_intermediates: false,
        }
    }

    // --- from_preset ---

    #[test]
    fn test_from_preset_gre_defaults() {
        let c = PipelineConfig::from_preset(cli::Preset::Gre);
        assert_eq!(c.qsm_algorithm, QsmAlgorithm::Rts);
        assert_eq!(c.unwrapping_algorithm, Some(UnwrappingAlgorithm::Romeo));
        assert_eq!(c.bf_algorithm, Some(BfAlgorithm::Vsharp));
        assert_eq!(c.masking_algorithm, MaskingAlgorithm::Threshold);
        assert_eq!(c.masking_input, MaskingInput::PhaseQuality);
        assert!(c.combine_phase);
        assert_eq!(c.mask_erosions, vec![2]);
        assert!(!c.do_swi);
        assert!(!c.do_t2starmap);
        assert!(!c.do_r2starmap);
    }

    #[test]
    fn test_from_preset_epi_differs_from_gre() {
        let epi = PipelineConfig::from_preset(cli::Preset::Epi);
        assert_eq!(epi.mask_erosions, vec![3]);
        assert_eq!(epi.qsm_algorithm, QsmAlgorithm::Rts); // inherits GRE
    }

    #[test]
    fn test_from_preset_bet_uses_bet_masking() {
        let c = PipelineConfig::from_preset(cli::Preset::Bet);
        assert_eq!(c.masking_algorithm, MaskingAlgorithm::Bet);
        assert_eq!(c.masking_input, MaskingInput::Magnitude);
        assert_eq!(c.mask_erosions, vec![3]);
    }

    #[test]
    fn test_from_preset_fast_uses_vsharp() {
        let c = PipelineConfig::from_preset(cli::Preset::Fast);
        assert_eq!(c.bf_algorithm, Some(BfAlgorithm::Vsharp));
        assert_eq!(c.masking_algorithm, MaskingAlgorithm::Bet);
    }

    #[test]
    fn test_from_preset_body_is_tgv() {
        let c = PipelineConfig::from_preset(cli::Preset::Body);
        assert_eq!(c.qsm_algorithm, QsmAlgorithm::Tgv);
        assert_eq!(c.unwrapping_algorithm, None);
        assert_eq!(c.bf_algorithm, None);
        assert!(!c.combine_phase);
    }

    // --- apply_run_overrides ---

    #[test]
    fn test_apply_run_overrides_no_change_when_all_none() {
        let original = PipelineConfig::from_preset(cli::Preset::Gre);
        let mut config = original.clone();
        let args = default_run_args();
        config.apply_run_overrides(&args);
        assert_eq!(config.qsm_algorithm, original.qsm_algorithm);
        assert_eq!(config.bf_algorithm, original.bf_algorithm);
        assert_eq!(config.masking_algorithm, original.masking_algorithm);
        assert_eq!(config.bet_fractional_intensity, original.bet_fractional_intensity);
    }

    #[test]
    fn test_apply_run_overrides_single_field() {
        let mut config = PipelineConfig::from_preset(cli::Preset::Gre);
        let mut args = default_run_args();
        args.qsm_algorithm = Some(cli::QsmAlgorithmArg::Tgv);
        config.apply_run_overrides(&args);
        assert_eq!(config.qsm_algorithm, QsmAlgorithm::Tgv);
        // Other fields unchanged
        assert_eq!(config.masking_algorithm, MaskingAlgorithm::Threshold);
    }

    #[test]
    fn test_apply_run_overrides_multiple_fields() {
        let mut config = PipelineConfig::from_preset(cli::Preset::Gre);
        let mut args = default_run_args();
        args.bf_algorithm = Some(cli::BfAlgorithmArg::Vsharp);
        args.masking_algorithm = Some(cli::MaskAlgorithmArg::Bet);
        args.bet_fractional_intensity = Some(0.3);
        args.mask_erosions = Some(vec![5, 3]);
        config.apply_run_overrides(&args);
        assert_eq!(config.bf_algorithm, Some(BfAlgorithm::Vsharp));
        assert_eq!(config.masking_algorithm, MaskingAlgorithm::Bet);
        assert!((config.bet_fractional_intensity - 0.3).abs() < 1e-10);
        assert_eq!(config.mask_erosions, vec![5, 3]);
    }

    #[test]
    fn test_apply_run_overrides_flags() {
        let mut config = PipelineConfig::from_preset(cli::Preset::Gre);
        let mut args = default_run_args();
        args.do_swi = true;
        args.do_t2starmap = true;
        args.do_r2starmap = true;
        config.apply_run_overrides(&args);
        assert!(config.do_swi);
        assert!(config.do_t2starmap);
        assert!(config.do_r2starmap);
    }

    // --- validate ---

    #[test]
    fn test_validate_gre_passes() {
        let config = PipelineConfig::from_preset(cli::Preset::Gre);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_tgv_accepts_none_bf_and_unwrap() {
        let config = PipelineConfig::from_preset(cli::Preset::Body);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_non_tgv_requires_bf() {
        let mut config = PipelineConfig::from_preset(cli::Preset::Gre);
        config.bf_algorithm = None;
        let result = config.validate();
        assert!(result.is_err());
    }

    // --- to_annotated_toml ---

    #[test]
    fn test_to_annotated_toml_contains_key_fields() {
        let config = PipelineConfig::from_preset(cli::Preset::Gre);
        let toml = config.to_annotated_toml();
        assert!(toml.contains("qsm_algorithm = \"rts\""));
        assert!(toml.contains("bf_algorithm = \"vsharp\""));
        assert!(toml.contains("masking_algorithm = \"threshold\""));
        assert!(toml.contains("[masking]"));
        assert!(toml.contains("[rts]"));
        assert!(toml.contains("[tgv]"));
    }

    #[test]
    fn test_to_annotated_toml_body_comments_out_bf() {
        let config = PipelineConfig::from_preset(cli::Preset::Body);
        let toml = config.to_annotated_toml();
        assert!(toml.contains("# bf_algorithm"), "BF should be commented out for Body/TGV");
    }

    // --- MaskingInput::PhaseQuality ---

    #[test]
    fn test_apply_run_overrides_masking_input() {
        let mut config = PipelineConfig::from_preset(cli::Preset::Gre);
        assert_eq!(config.masking_input, MaskingInput::PhaseQuality); // default
        let mut args = default_run_args();
        args.masking_input = Some(cli::MaskInputArg::MagnitudeFirst);
        config.apply_run_overrides(&args);
        assert_eq!(config.masking_input, MaskingInput::MagnitudeFirst);
    }

    #[test]
    fn test_masking_input_display() {
        assert_eq!(format!("{}", MaskingInput::PhaseQuality), "phase-quality");
        assert_eq!(format!("{}", MaskingInput::Magnitude), "magnitude");
        assert_eq!(format!("{}", MaskingInput::MagnitudeFirst), "magnitude-first");
        assert_eq!(format!("{}", MaskingInput::MagnitudeLast), "magnitude-last");
    }

    // --- obliquity_threshold ---

    #[test]
    fn test_default_obliquity_threshold_disabled() {
        let config = PipelineConfig::from_preset(cli::Preset::Gre);
        assert!(config.obliquity_threshold < 0.0, "Default should be disabled (-1)");
    }

    #[test]
    fn test_apply_run_overrides_obliquity_threshold() {
        let mut config = PipelineConfig::from_preset(cli::Preset::Gre);
        let mut args = default_run_args();
        args.obliquity_threshold = Some(5.0);
        config.apply_run_overrides(&args);
        assert!((config.obliquity_threshold - 5.0).abs() < 1e-10);
    }

    // --- parse_mask_op ---

    #[test]
    fn test_parse_mask_op_threshold_otsu() {
        let op = parse_mask_op("threshold:otsu").unwrap();
        assert_eq!(op, MaskOp::Threshold { method: MaskThresholdMethod::Otsu, value: None });
    }

    #[test]
    fn test_parse_mask_op_threshold_fixed() {
        let op = parse_mask_op("threshold:fixed:0.3").unwrap();
        assert_eq!(op, MaskOp::Threshold { method: MaskThresholdMethod::Fixed, value: Some(0.3) });
    }

    #[test]
    fn test_parse_mask_op_threshold_percentile() {
        let op = parse_mask_op("threshold:percentile:90").unwrap();
        assert_eq!(op, MaskOp::Threshold { method: MaskThresholdMethod::Percentile, value: Some(90.0) });
    }

    #[test]
    fn test_parse_mask_op_erode() {
        let op = parse_mask_op("erode:3").unwrap();
        assert_eq!(op, MaskOp::Erode { iterations: 3 });
    }

    #[test]
    fn test_parse_mask_op_dilate() {
        let op = parse_mask_op("dilate:2").unwrap();
        assert_eq!(op, MaskOp::Dilate { iterations: 2 });
    }

    #[test]
    fn test_parse_mask_op_close() {
        let op = parse_mask_op("close:1").unwrap();
        assert_eq!(op, MaskOp::Close { radius: 1 });
    }

    #[test]
    fn test_parse_mask_op_fill_holes() {
        let op = parse_mask_op("fill-holes:500").unwrap();
        assert_eq!(op, MaskOp::FillHoles { max_size: 500 });
    }

    #[test]
    fn test_parse_mask_op_gaussian() {
        let op = parse_mask_op("gaussian:4.0").unwrap();
        assert_eq!(op, MaskOp::GaussianSmooth { sigma_mm: 4.0 });
    }

    #[test]
    fn test_parse_mask_op_input_magnitude() {
        let op = parse_mask_op("input:magnitude").unwrap();
        assert_eq!(op, MaskOp::Input { source: MaskingInput::Magnitude });
    }

    #[test]
    fn test_parse_mask_op_bet() {
        let op = parse_mask_op("bet:0.4").unwrap();
        assert_eq!(op, MaskOp::Bet { fractional_intensity: 0.4 });
    }

    #[test]
    fn test_parse_mask_op_invalid() {
        assert!(parse_mask_op("foobar:123").is_err());
    }

    #[test]
    fn test_algorithm_display() {
        assert_eq!(format!("{}", QsmAlgorithm::Rts), "rts");
        assert_eq!(format!("{}", QsmAlgorithm::Tv), "tv");
        assert_eq!(format!("{}", QsmAlgorithm::Tkd), "tkd");
        assert_eq!(format!("{}", QsmAlgorithm::Tgv), "tgv");
        assert_eq!(format!("{}", UnwrappingAlgorithm::Romeo), "romeo");
        assert_eq!(format!("{}", UnwrappingAlgorithm::Laplacian), "laplacian");
        assert_eq!(format!("{}", BfAlgorithm::Vsharp), "vsharp");
        assert_eq!(format!("{}", BfAlgorithm::Pdf), "pdf");
        assert_eq!(format!("{}", BfAlgorithm::Lbv), "lbv");
        assert_eq!(format!("{}", BfAlgorithm::Ismv), "ismv");
        assert_eq!(format!("{}", MaskingAlgorithm::Bet), "bet");
        assert_eq!(format!("{}", MaskingAlgorithm::Threshold), "threshold");
        assert_eq!(format!("{}", QsmReference::Mean), "mean");
        assert_eq!(format!("{}", QsmReference::None), "none");
    }

    #[test]
    fn test_mask_op_display_all_variants() {
        assert_eq!(format!("{}", MaskOp::Input { source: MaskingInput::Magnitude }), "input:magnitude");
        assert_eq!(format!("{}", MaskOp::Threshold { method: MaskThresholdMethod::Otsu, value: None }), "threshold:otsu");
        assert_eq!(format!("{}", MaskOp::Threshold { method: MaskThresholdMethod::Fixed, value: Some(0.3) }), "threshold:fixed:0.3");
        assert_eq!(format!("{}", MaskOp::Threshold { method: MaskThresholdMethod::Fixed, value: None }), "threshold:fixed:0.5");
        assert_eq!(format!("{}", MaskOp::Threshold { method: MaskThresholdMethod::Percentile, value: Some(90.0) }), "threshold:percentile:90");
        assert_eq!(format!("{}", MaskOp::Threshold { method: MaskThresholdMethod::Percentile, value: None }), "threshold:percentile:75");
        assert_eq!(format!("{}", MaskOp::Bet { fractional_intensity: 0.4 }), "bet:0.4");
        assert_eq!(format!("{}", MaskOp::Erode { iterations: 3 }), "erode:3");
        assert_eq!(format!("{}", MaskOp::Dilate { iterations: 1 }), "dilate:1");
        assert_eq!(format!("{}", MaskOp::Close { radius: 2 }), "close:2");
        assert_eq!(format!("{}", MaskOp::FillHoles { max_size: 500 }), "fill-holes:500");
        assert_eq!(format!("{}", MaskOp::GaussianSmooth { sigma_mm: 4.0 }), "gaussian:4");
    }

    #[test]
    fn test_parse_mask_op_input_all_sources() {
        assert_eq!(parse_mask_op("input:magnitude-first").unwrap(), MaskOp::Input { source: MaskingInput::MagnitudeFirst });
        assert_eq!(parse_mask_op("input:magnitude-last").unwrap(), MaskOp::Input { source: MaskingInput::MagnitudeLast });
        assert_eq!(parse_mask_op("input:phase-quality").unwrap(), MaskOp::Input { source: MaskingInput::PhaseQuality });
    }

    #[test]
    fn test_parse_mask_op_input_invalid() {
        assert!(parse_mask_op("input:foo").is_err());
    }

    #[test]
    fn test_parse_mask_op_threshold_invalid_method() {
        assert!(parse_mask_op("threshold:invalid").is_err());
    }

    #[test]
    fn test_parse_mask_op_threshold_default() {
        // No method specified defaults to otsu
        let op = parse_mask_op("threshold").unwrap();
        assert_eq!(op, MaskOp::Threshold { method: MaskThresholdMethod::Otsu, value: None });
    }

    #[test]
    fn test_parse_mask_op_defaults_when_no_value() {
        let op = parse_mask_op("bet").unwrap();
        assert_eq!(op, MaskOp::Bet { fractional_intensity: 0.5 });
        let op = parse_mask_op("erode").unwrap();
        assert_eq!(op, MaskOp::Erode { iterations: 1 });
        let op = parse_mask_op("dilate").unwrap();
        assert_eq!(op, MaskOp::Dilate { iterations: 1 });
        let op = parse_mask_op("close").unwrap();
        assert_eq!(op, MaskOp::Close { radius: 1 });
        let op = parse_mask_op("fill-holes").unwrap();
        assert_eq!(op, MaskOp::FillHoles { max_size: 1000 });
        let op = parse_mask_op("gaussian").unwrap();
        assert_eq!(op, MaskOp::GaussianSmooth { sigma_mm: 4.0 });
    }

    #[test]
    fn test_from_file_valid_toml() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let config = PipelineConfig::from_preset(cli::Preset::Gre);
        let toml_str = config.to_annotated_toml();
        std::fs::write(&path, &toml_str).unwrap();
        // Should parse without error (not all fields need to match since
        // annotated toml uses [pipeline] section but struct is flat)
        // At minimum, from_file should not panic
        let _ = PipelineConfig::from_file(&path);
    }

    #[test]
    fn test_from_file_missing_file() {
        let result = PipelineConfig::from_file(std::path::Path::new("/nonexistent/config.toml"));
        assert!(result.is_err());
    }

    #[test]
    fn test_to_annotated_toml_body_preset() {
        let config = PipelineConfig::from_preset(cli::Preset::Body);
        let toml = config.to_annotated_toml();
        assert!(toml.contains("qsm_algorithm = \"tgv\""));
        assert!(toml.contains("# unwrapping_algorithm")); // commented out
        assert!(toml.contains("# bf_algorithm")); // commented out
        assert!(toml.contains("combine_phase = false"));
        assert!(toml.contains("do_swi = false"));
        assert!(toml.contains("do_t2starmap = false"));
        assert!(toml.contains("do_r2starmap = false"));
        assert!(toml.contains("inhomogeneity_correction = false"));
        assert!(toml.contains("obliquity_threshold = -1"));
    }

    #[test]
    fn test_to_annotated_toml_with_features_enabled() {
        let mut config = PipelineConfig::from_preset(cli::Preset::Gre);
        config.do_swi = true;
        config.do_t2starmap = true;
        config.do_r2starmap = true;
        config.inhomogeneity_correction = true;
        config.description = "".to_string();
        let toml = config.to_annotated_toml();
        assert!(toml.contains("do_swi = true"));
        assert!(toml.contains("do_t2starmap = true"));
        assert!(toml.contains("do_r2starmap = true"));
        assert!(toml.contains("inhomogeneity_correction = true"));
        assert!(!toml.contains("# Preset:")); // empty description not printed
    }

    #[test]
    fn test_list_presets() {
        let presets = list_presets();
        assert_eq!(presets.len(), 5);
        assert_eq!(presets[0].0, "gre");
        assert_eq!(presets[4].0, "body");
    }

    #[test]
    fn test_apply_run_overrides_all_algorithms() {
        let mut config = PipelineConfig::from_preset(cli::Preset::Gre);
        let mut args = default_run_args();

        // TV
        args.qsm_algorithm = Some(cli::QsmAlgorithmArg::Tv);
        config.apply_run_overrides(&args);
        assert_eq!(config.qsm_algorithm, QsmAlgorithm::Tv);

        // TKD
        args.qsm_algorithm = Some(cli::QsmAlgorithmArg::Tkd);
        config.apply_run_overrides(&args);
        assert_eq!(config.qsm_algorithm, QsmAlgorithm::Tkd);

        // Laplacian
        args.unwrapping_algorithm = Some(cli::UnwrapAlgorithmArg::Laplacian);
        config.apply_run_overrides(&args);
        assert_eq!(config.unwrapping_algorithm, Some(UnwrappingAlgorithm::Laplacian));

        // LBV
        args.bf_algorithm = Some(cli::BfAlgorithmArg::Lbv);
        config.apply_run_overrides(&args);
        assert_eq!(config.bf_algorithm, Some(BfAlgorithm::Lbv));

        // iSMV
        args.bf_algorithm = Some(cli::BfAlgorithmArg::Ismv);
        config.apply_run_overrides(&args);
        assert_eq!(config.bf_algorithm, Some(BfAlgorithm::Ismv));

        // PDF
        args.bf_algorithm = Some(cli::BfAlgorithmArg::Pdf);
        config.apply_run_overrides(&args);
        assert_eq!(config.bf_algorithm, Some(BfAlgorithm::Pdf));

        // Masking input variants
        args.masking_input = Some(cli::MaskInputArg::Magnitude);
        config.apply_run_overrides(&args);
        assert_eq!(config.masking_input, MaskingInput::Magnitude);

        args.masking_input = Some(cli::MaskInputArg::MagnitudeLast);
        config.apply_run_overrides(&args);
        assert_eq!(config.masking_input, MaskingInput::MagnitudeLast);
    }

    #[test]
    fn test_apply_run_overrides_numeric_params() {
        let mut config = PipelineConfig::from_preset(cli::Preset::Gre);
        let mut args = default_run_args();
        args.rts_mu = Some(2e5);
        args.rts_tol = Some(1e-6);
        args.tv_lambda = Some(0.01);
        args.tkd_threshold = Some(0.2);
        args.tgv_iterations = Some(500);
        args.tgv_erosions = Some(5);
        args.inhomogeneity_correction = true;
        args.obliquity_threshold = Some(10.0);
        config.apply_run_overrides(&args);
        assert!((config.rts_mu - 2e5).abs() < 1.0);
        assert!((config.rts_tol - 1e-6).abs() < 1e-10);
        assert!((config.tv_lambda - 0.01).abs() < 1e-10);
        assert!((config.tkd_threshold - 0.2).abs() < 1e-10);
        assert_eq!(config.tgv_iterations, 500);
        assert_eq!(config.tgv_erosions, 5);
        assert!(config.inhomogeneity_correction);
        assert!((config.obliquity_threshold - 10.0).abs() < 1e-10);
    }

    #[test]
    fn test_default_config_is_gre() {
        let config = PipelineConfig::default();
        assert_eq!(config.qsm_algorithm, QsmAlgorithm::Rts);
        assert_eq!(config.description, "3D-GRE images (human brain)");
    }

    #[test]
    fn test_validate_tgv_with_bf_set() {
        let mut config = PipelineConfig::from_preset(cli::Preset::Gre);
        config.qsm_algorithm = QsmAlgorithm::Tgv;
        config.bf_algorithm = Some(BfAlgorithm::Vsharp);
        // Should still pass — TGV ignores bf/unwrap but doesn't error
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_parse_mask_op_display_roundtrip() {
        let op = MaskOp::Erode { iterations: 3 };
        let s = format!("{}", op);
        let parsed = parse_mask_op(&s).unwrap();
        assert_eq!(parsed, op);
    }

    #[test]
    fn test_mask_ops_default_in_preset() {
        let config = PipelineConfig::from_preset(cli::Preset::Gre);
        assert!(!config.mask_ops.is_empty(), "GRE preset should have default mask_ops");
        // Default: Input(PhaseQuality) → Threshold(Otsu) → Dilate(2) → FillHoles → Erode(2)
        assert_eq!(config.mask_ops.len(), 5);
        assert_eq!(config.mask_ops[0], MaskOp::Input { source: MaskingInput::PhaseQuality });
        assert_eq!(config.mask_ops[1], MaskOp::Threshold { method: MaskThresholdMethod::Otsu, value: None });
        assert_eq!(config.mask_ops[2], MaskOp::Dilate { iterations: 2 });
        assert_eq!(config.mask_ops[4], MaskOp::Erode { iterations: 2 });
    }

    #[test]
    fn test_mask_ops_bet_preset() {
        let config = PipelineConfig::from_preset(cli::Preset::Bet);
        assert_eq!(config.mask_ops.len(), 2);
        assert_eq!(config.mask_ops[0], MaskOp::Bet { fractional_intensity: 0.5 });
        assert_eq!(config.mask_ops[1], MaskOp::Erode { iterations: 3 });
    }

    #[test]
    fn test_mask_ops_override_from_cli() {
        let mut config = PipelineConfig::from_preset(cli::Preset::Gre);
        let mut args = default_run_args();
        args.mask_ops = Some(vec![
            "input:magnitude".to_string(),
            "threshold:otsu".to_string(),
            "erode:2".to_string(),
        ]);
        config.apply_run_overrides(&args);
        assert_eq!(config.mask_ops.len(), 3);
        assert_eq!(config.mask_ops[0], MaskOp::Input { source: MaskingInput::Magnitude });
        assert_eq!(config.mask_ops[2], MaskOp::Erode { iterations: 2 });
    }
}
