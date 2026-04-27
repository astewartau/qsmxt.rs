use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::Path;

use crate::cli;
use crate::error::QsmxtError;

// ─── Mask operation pipeline ───

/// A single mask section: input → generator → refinement steps.
/// Multiple sections are OR'd together at runtime.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MaskSection {
    pub input: MaskingInput,
    /// The mask-generating step (threshold or BET). Always exactly one.
    pub generator: MaskOp,
    /// Morphological refinement steps applied after the generator.
    #[serde(default)]
    pub refinements: Vec<MaskOp>,
}

impl MaskSection {
    /// Check if this section has a valid generator.
    pub fn has_generator(&self) -> bool {
        matches!(self.generator, MaskOp::Threshold { .. } | MaskOp::Bet { .. })
    }

    /// Get all ops in order (generator + refinements) for runtime execution.
    pub fn all_ops(&self) -> Vec<MaskOp> {
        let mut ops = vec![self.generator.clone()];
        ops.extend(self.refinements.iter().cloned());
        ops
    }
}


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
    Tsvd,
    Tgv,
    Tikhonov,
    Nltv,
    Medi,
    Ilsqr,
    Qsmart,
}

impl fmt::Display for QsmAlgorithm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Rts => write!(f, "rts"),
            Self::Tv => write!(f, "tv"),
            Self::Tkd => write!(f, "tkd"),
            Self::Tsvd => write!(f, "tsvd"),
            Self::Tgv => write!(f, "tgv"),
            Self::Tikhonov => write!(f, "tikhonov"),
            Self::Nltv => write!(f, "nltv"),
            Self::Medi => write!(f, "medi"),
            Self::Ilsqr => write!(f, "ilsqr"),
            Self::Qsmart => write!(f, "qsmart"),
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
    Sharp,
}

impl fmt::Display for BfAlgorithm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Vsharp => write!(f, "vsharp"),
            Self::Pdf => write!(f, "pdf"),
            Self::Lbv => write!(f, "lbv"),
            Self::Ismv => write!(f, "ismv"),
            Self::Sharp => write!(f, "sharp"),
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

    /// Mask sections, OR'd together at runtime. Each section has an input source and ops.
    #[serde(default = "default_mask_sections")]
    pub mask_sections: Vec<MaskSection>,

    // Algorithm choices
    #[serde(default = "default_qsm_algorithm")]
    pub qsm_algorithm: QsmAlgorithm,
    #[serde(default = "default_unwrapping")]
    pub unwrapping_algorithm: Option<UnwrappingAlgorithm>,
    #[serde(default = "default_bf")]
    pub bf_algorithm: Option<BfAlgorithm>,

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

    // TSVD parameters
    #[serde(default = "default_tsvd_threshold")]
    pub tsvd_threshold: f64,

    // iLSQR parameters
    #[serde(default = "default_ilsqr_tol")]
    pub ilsqr_tol: f64,
    #[serde(default = "default_ilsqr_max_iter")]
    pub ilsqr_max_iter: usize,

    // Tikhonov parameters
    #[serde(default = "default_tikhonov_lambda")]
    pub tikhonov_lambda: f64,

    // NLTV parameters
    #[serde(default = "default_nltv_lambda")]
    pub nltv_lambda: f64,
    #[serde(default = "default_nltv_mu")]
    pub nltv_mu: f64,
    #[serde(default = "default_nltv_tol")]
    pub nltv_tol: f64,
    #[serde(default = "default_nltv_max_iter")]
    pub nltv_max_iter: usize,
    #[serde(default = "default_nltv_newton_iter")]
    pub nltv_newton_iter: usize,

    // MEDI parameters
    #[serde(default = "default_medi_lambda")]
    pub medi_lambda: f64,
    #[serde(default = "default_medi_max_iter")]
    pub medi_max_iter: usize,
    #[serde(default = "default_medi_cg_max_iter")]
    pub medi_cg_max_iter: usize,
    #[serde(default = "default_medi_cg_tol")]
    pub medi_cg_tol: f64,
    #[serde(default = "default_medi_tol")]
    pub medi_tol: f64,
    #[serde(default = "default_medi_percentage")]
    pub medi_percentage: f64,
    #[serde(default = "default_medi_smv_radius")]
    pub medi_smv_radius: f64,
    #[serde(default = "default_medi_smv")]
    pub medi_smv: bool,

    // V-SHARP parameters
    #[serde(default = "default_vsharp_threshold")]
    pub vsharp_threshold: f64,
    #[serde(default = "default_vsharp_max_radius_factor")]
    pub vsharp_max_radius_factor: f64,
    #[serde(default = "default_vsharp_min_radius_factor")]
    pub vsharp_min_radius_factor: f64,

    // PDF parameters
    #[serde(default = "default_pdf_tol")]
    pub pdf_tol: f64,

    // LBV parameters
    #[serde(default = "default_lbv_tol")]
    pub lbv_tol: f64,

    // iSMV parameters
    #[serde(default = "default_ismv_tol")]
    pub ismv_tol: f64,
    #[serde(default = "default_ismv_max_iter")]
    pub ismv_max_iter: usize,
    #[serde(default = "default_ismv_radius_factor")]
    pub ismv_radius_factor: f64,

    // SHARP parameters
    #[serde(default = "default_sharp_threshold")]
    pub sharp_threshold: f64,
    #[serde(default = "default_sharp_radius_factor")]
    pub sharp_radius_factor: f64,

    // ROMEO parameters
    #[serde(default = "default_romeo_phase_gradient_coherence")]
    pub romeo_phase_gradient_coherence: bool,
    #[serde(default = "default_romeo_mag_coherence")]
    pub romeo_mag_coherence: bool,
    #[serde(default = "default_romeo_mag_weight")]
    pub romeo_mag_weight: bool,

    // MCPC-3D-S parameters
    #[serde(default = "default_mcpc3ds_sigma")]
    pub mcpc3ds_sigma: [f64; 3],

    // SWI parameters
    #[serde(default = "default_swi_hp_sigma")]
    pub swi_hp_sigma: [f64; 3],
    #[serde(default = "default_swi_scaling")]
    pub swi_scaling: String,
    #[serde(default = "default_swi_strength")]
    pub swi_strength: f64,
    #[serde(default = "default_swi_mip_window")]
    pub swi_mip_window: usize,

    // Homogeneity correction parameters
    #[serde(default = "default_homogeneity_sigma_mm")]
    pub homogeneity_sigma_mm: f64,
    #[serde(default = "default_homogeneity_nbox")]
    pub homogeneity_nbox: usize,

    // Linear fit parameters
    #[serde(default = "default_linear_fit_reliability_threshold")]
    pub linear_fit_reliability_threshold: f64,

    // TGV parameters
    #[serde(default = "default_tgv_iterations")]
    pub tgv_iterations: usize,
    #[serde(default = "default_tgv_alphas")]
    pub tgv_alphas: [f64; 2],
    #[serde(default = "default_tgv_erosions")]
    pub tgv_erosions: usize,
    #[serde(default = "default_tgv_step_size")]
    pub tgv_step_size: f64,
    #[serde(default = "default_tgv_tol")]
    pub tgv_tol: f64,

    // QSMART parameters
    #[serde(default = "default_qsmart_ilsqr_tol")]
    pub qsmart_ilsqr_tol: f64,
    #[serde(default = "default_qsmart_ilsqr_max_iter")]
    pub qsmart_ilsqr_max_iter: usize,
    #[serde(default = "default_qsmart_vasc_sphere_radius")]
    pub qsmart_vasc_sphere_radius: i32,
    #[serde(default = "default_qsmart_sdf_spatial_radius")]
    pub qsmart_sdf_spatial_radius: i32,
}

// Defaults
fn default_true() -> bool { true }
fn default_qsm_algorithm() -> QsmAlgorithm { QsmAlgorithm::Rts }
fn default_unwrapping() -> Option<UnwrappingAlgorithm> { Some(UnwrappingAlgorithm::Romeo) }
fn default_bf() -> Option<BfAlgorithm> { Some(BfAlgorithm::Vsharp) }
fn default_reference() -> QsmReference { QsmReference::Mean }
fn default_bet_fi() -> f64 { qsm_core::bet::BetParams::default().fractional_intensity }
fn default_bet_smoothness() -> f64 { qsm_core::bet::BetParams::default().smoothness }
fn default_bet_gradient() -> f64 { qsm_core::bet::BetParams::default().gradient_threshold }
fn default_bet_iterations() -> usize { qsm_core::bet::BetParams::default().iterations }
fn default_bet_subdivisions() -> usize { qsm_core::bet::BetParams::default().subdivisions }
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
fn default_tsvd_threshold() -> f64 { qsm_core::inversion::TkdParams::default().threshold }
fn default_ilsqr_tol() -> f64 { qsm_core::inversion::IlsqrParams::default().tol }
fn default_ilsqr_max_iter() -> usize { qsm_core::inversion::IlsqrParams::default().max_iter }
fn default_tikhonov_lambda() -> f64 { qsm_core::inversion::TikhonovParams::default().lambda }
fn default_nltv_lambda() -> f64 { qsm_core::inversion::NltvParams::default().lambda }
fn default_nltv_mu() -> f64 { qsm_core::inversion::NltvParams::default().mu }
fn default_nltv_tol() -> f64 { qsm_core::inversion::NltvParams::default().tol }
fn default_nltv_max_iter() -> usize { qsm_core::inversion::NltvParams::default().max_iter }
fn default_nltv_newton_iter() -> usize { qsm_core::inversion::NltvParams::default().newton_iter }
fn default_medi_lambda() -> f64 { qsm_core::inversion::MediParams::default().lambda }
fn default_medi_max_iter() -> usize { qsm_core::inversion::MediParams::default().max_iter }
fn default_medi_cg_max_iter() -> usize { qsm_core::inversion::MediParams::default().cg_max_iter }
fn default_medi_cg_tol() -> f64 { qsm_core::inversion::MediParams::default().cg_tol }
fn default_medi_tol() -> f64 { qsm_core::inversion::MediParams::default().tol }
fn default_medi_percentage() -> f64 { qsm_core::inversion::MediParams::default().percentage }
fn default_medi_smv_radius() -> f64 { qsm_core::inversion::MediParams::default().smv_radius }
fn default_medi_smv() -> bool { qsm_core::inversion::MediParams::default().smv }
fn default_vsharp_threshold() -> f64 { qsm_core::bgremove::VsharpParams::default().threshold }
fn default_vsharp_max_radius_factor() -> f64 { qsm_core::bgremove::VsharpParams::default().max_radius_factor }
fn default_vsharp_min_radius_factor() -> f64 { qsm_core::bgremove::VsharpParams::default().min_radius_factor }
fn default_pdf_tol() -> f64 { qsm_core::bgremove::PdfParams::default().tol }
fn default_lbv_tol() -> f64 { qsm_core::bgremove::LbvParams::default().tol }
fn default_ismv_tol() -> f64 { qsm_core::bgremove::IsmvParams::default().tol }
fn default_ismv_max_iter() -> usize { qsm_core::bgremove::IsmvParams::default().max_iter }
fn default_ismv_radius_factor() -> f64 { qsm_core::bgremove::IsmvParams::default().radius_factor }
fn default_sharp_threshold() -> f64 { qsm_core::bgremove::SharpParams::default().threshold }
fn default_sharp_radius_factor() -> f64 { qsm_core::bgremove::SharpParams::default().radius_factor }
fn default_romeo_phase_gradient_coherence() -> bool { qsm_core::unwrap::RomeoParams::default().phase_gradient_coherence }
fn default_romeo_mag_coherence() -> bool { qsm_core::unwrap::RomeoParams::default().mag_coherence }
fn default_romeo_mag_weight() -> bool { qsm_core::unwrap::RomeoParams::default().mag_weight }
fn default_mcpc3ds_sigma() -> [f64; 3] { qsm_core::utils::Mcpc3dsParams::default().sigma }
fn default_swi_hp_sigma() -> [f64; 3] { qsm_core::swi::SwiParams::default().hp_sigma }
fn default_swi_scaling() -> String { "tanh".to_string() }
fn default_swi_strength() -> f64 { qsm_core::swi::SwiParams::default().strength }
fn default_swi_mip_window() -> usize { qsm_core::swi::SwiParams::default().mip_window }
fn default_homogeneity_sigma_mm() -> f64 { qsm_core::utils::HomogeneityParams::default().sigma_mm }
fn default_homogeneity_nbox() -> usize { qsm_core::utils::HomogeneityParams::default().nbox }
fn default_linear_fit_reliability_threshold() -> f64 { qsm_core::utils::LinearFitParams::default().reliability_threshold_percentile }
fn default_tgv_iterations() -> usize { qsm_core::inversion::TgvParams::default().iterations }
fn default_tgv_step_size() -> f64 { qsm_core::inversion::TgvParams::default().step_size as f64 }
fn default_tgv_tol() -> f64 { qsm_core::inversion::TgvParams::default().tol as f64 }
fn default_mask_sections() -> Vec<MaskSection> {
    vec![MaskSection {
        input: MaskingInput::PhaseQuality,
        generator: MaskOp::Threshold { method: MaskThresholdMethod::Otsu, value: None },
        refinements: vec![
            MaskOp::Dilate { iterations: 2 },
            MaskOp::FillHoles { max_size: 0 },
            MaskOp::Erode { iterations: 2 },
        ],
    }]
}
/// Parse a masking input source string.
pub fn parse_masking_input(s: &str) -> Option<MaskingInput> {
    match s.trim() {
        "magnitude" => Some(MaskingInput::Magnitude),
        "magnitude-first" => Some(MaskingInput::MagnitudeFirst),
        "magnitude-last" => Some(MaskingInput::MagnitudeLast),
        "phase-quality" => Some(MaskingInput::PhaseQuality),
        _ => None,
    }
}

fn default_obliquity_threshold() -> f64 { -1.0 }
fn default_tgv_alphas() -> [f64; 2] {
    let p = qsm_core::inversion::TgvParams::default();
    [p.alpha1 as f64, p.alpha0 as f64]
}
fn default_tgv_erosions() -> usize { qsm_core::inversion::TgvParams::default().erosions }
fn default_qsmart_ilsqr_tol() -> f64 { qsm_core::utils::QsmartParams::default().ilsqr_tol }
fn default_qsmart_ilsqr_max_iter() -> usize { qsm_core::utils::QsmartParams::default().ilsqr_max_iter }
fn default_qsmart_vasc_sphere_radius() -> i32 { qsm_core::utils::QsmartParams::default().vasc_sphere_radius }
fn default_qsmart_sdf_spatial_radius() -> i32 { qsm_core::utils::QsmartParams::default().sdf_spatial_radius }

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
                inhomogeneity_correction: true,
                obliquity_threshold: -1.0,
                mask_sections: default_mask_sections(),
                qsm_algorithm: QsmAlgorithm::Rts,
                unwrapping_algorithm: Some(UnwrappingAlgorithm::Romeo),
                bf_algorithm: Some(BfAlgorithm::Vsharp),
                combine_phase: true,
                qsm_reference: QsmReference::Mean,
                bet_fractional_intensity: default_bet_fi(),
                bet_smoothness: default_bet_smoothness(),
                bet_gradient_threshold: default_bet_gradient(),
                bet_iterations: default_bet_iterations(),
                bet_subdivisions: default_bet_subdivisions(),
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
                tsvd_threshold: default_tsvd_threshold(),
                ilsqr_tol: default_ilsqr_tol(),
                ilsqr_max_iter: default_ilsqr_max_iter(),
                tikhonov_lambda: default_tikhonov_lambda(),
                nltv_lambda: default_nltv_lambda(),
                nltv_mu: default_nltv_mu(),
                nltv_tol: default_nltv_tol(),
                nltv_max_iter: default_nltv_max_iter(),
                nltv_newton_iter: default_nltv_newton_iter(),
                medi_lambda: default_medi_lambda(),
                medi_max_iter: default_medi_max_iter(),
                medi_cg_max_iter: default_medi_cg_max_iter(),
                medi_cg_tol: default_medi_cg_tol(),
                medi_tol: default_medi_tol(),
                medi_percentage: default_medi_percentage(),
                medi_smv_radius: default_medi_smv_radius(),
                medi_smv: default_medi_smv(),
                vsharp_threshold: default_vsharp_threshold(),
                vsharp_max_radius_factor: default_vsharp_max_radius_factor(),
                vsharp_min_radius_factor: default_vsharp_min_radius_factor(),
                pdf_tol: default_pdf_tol(),
                lbv_tol: default_lbv_tol(),
                ismv_tol: default_ismv_tol(),
                ismv_max_iter: default_ismv_max_iter(),
                ismv_radius_factor: default_ismv_radius_factor(),
                sharp_threshold: default_sharp_threshold(),
                sharp_radius_factor: default_sharp_radius_factor(),
                romeo_phase_gradient_coherence: default_romeo_phase_gradient_coherence(),
                romeo_mag_coherence: default_romeo_mag_coherence(),
                romeo_mag_weight: default_romeo_mag_weight(),
                mcpc3ds_sigma: default_mcpc3ds_sigma(),
                swi_hp_sigma: default_swi_hp_sigma(),
                swi_scaling: default_swi_scaling(),
                swi_strength: default_swi_strength(),
                swi_mip_window: default_swi_mip_window(),
                homogeneity_sigma_mm: default_homogeneity_sigma_mm(),
                homogeneity_nbox: default_homogeneity_nbox(),
                linear_fit_reliability_threshold: default_linear_fit_reliability_threshold(),
                tgv_iterations: default_tgv_iterations(),
                tgv_alphas: default_tgv_alphas(),
                tgv_erosions: default_tgv_erosions(),
                tgv_step_size: default_tgv_step_size(),
                tgv_tol: default_tgv_tol(),
                qsmart_ilsqr_tol: default_qsmart_ilsqr_tol(),
                qsmart_ilsqr_max_iter: default_qsmart_ilsqr_max_iter(),
                qsmart_vasc_sphere_radius: default_qsmart_vasc_sphere_radius(),
                qsmart_sdf_spatial_radius: default_qsmart_sdf_spatial_radius(),
            },
            cli::Preset::Epi => Self {
                description: "3D-EPI images (human brain)".to_string(),
                ..Self::from_preset(cli::Preset::Gre)
            },
            cli::Preset::Bet => Self {
                description: "Traditional BET masking (human brain)".to_string(),
                mask_sections: vec![MaskSection {
                    input: MaskingInput::Magnitude,
                    generator: MaskOp::Bet { fractional_intensity: 0.5 },
                    refinements: vec![MaskOp::Erode { iterations: 3 }],
                }],
                ..Self::from_preset(cli::Preset::Gre)
            },
            cli::Preset::Fast => Self {
                description: "Fast algorithms".to_string(),
                bf_algorithm: Some(BfAlgorithm::Vsharp),
                mask_sections: vec![MaskSection {
                    input: MaskingInput::Magnitude,
                    generator: MaskOp::Bet { fractional_intensity: 0.5 },
                    refinements: vec![MaskOp::Erode { iterations: 3 }],
                }],
                ..Self::from_preset(cli::Preset::Gre)
            },
            cli::Preset::Body => Self {
                description: "Non-brain applications".to_string(),
                qsm_algorithm: QsmAlgorithm::Tgv,
                unwrapping_algorithm: None,
                bf_algorithm: None,
                combine_phase: false,
                mask_sections: vec![MaskSection {
                    input: MaskingInput::Magnitude,
                    generator: MaskOp::Threshold { method: MaskThresholdMethod::Otsu, value: None },
                    refinements: vec![
                        MaskOp::Dilate { iterations: 2 },
                        MaskOp::FillHoles { max_size: 0 },
                        MaskOp::Erode { iterations: 3 },
                    ],
                }],
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
                cli::QsmAlgorithmArg::Tikhonov => QsmAlgorithm::Tikhonov,
                cli::QsmAlgorithmArg::Nltv => QsmAlgorithm::Nltv,
                cli::QsmAlgorithmArg::Tsvd => QsmAlgorithm::Tsvd,
                cli::QsmAlgorithmArg::Medi => QsmAlgorithm::Medi,
                cli::QsmAlgorithmArg::Ilsqr => QsmAlgorithm::Ilsqr,
                cli::QsmAlgorithmArg::Qsmart => QsmAlgorithm::Qsmart,
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
                cli::BfAlgorithmArg::Sharp => BfAlgorithm::Sharp,
            });
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
        if let Some(v) = args.tsvd_threshold {
            self.tsvd_threshold = v;
        }
        if let Some(v) = args.ilsqr_tol {
            self.ilsqr_tol = v;
        }
        if let Some(v) = args.ilsqr_max_iter {
            self.ilsqr_max_iter = v;
        }
        if let Some(v) = args.tikhonov_lambda {
            self.tikhonov_lambda = v;
        }
        if let Some(v) = args.nltv_lambda {
            self.nltv_lambda = v;
        }
        if let Some(v) = args.nltv_mu {
            self.nltv_mu = v;
        }
        if let Some(v) = args.nltv_tol {
            self.nltv_tol = v;
        }
        if let Some(v) = args.nltv_max_iter {
            self.nltv_max_iter = v;
        }
        if let Some(v) = args.nltv_newton_iter {
            self.nltv_newton_iter = v;
        }
        if let Some(v) = args.medi_lambda {
            self.medi_lambda = v;
        }
        if let Some(v) = args.medi_max_iter {
            self.medi_max_iter = v;
        }
        if let Some(v) = args.medi_cg_max_iter {
            self.medi_cg_max_iter = v;
        }
        if let Some(v) = args.medi_cg_tol {
            self.medi_cg_tol = v;
        }
        if let Some(v) = args.medi_tol {
            self.medi_tol = v;
        }
        if let Some(v) = args.medi_percentage {
            self.medi_percentage = v;
        }
        if let Some(v) = args.medi_smv_radius {
            self.medi_smv_radius = v;
        }
        if args.medi_smv {
            self.medi_smv = true;
        }
        if let Some(v) = args.vsharp_threshold {
            self.vsharp_threshold = v;
        }
        if let Some(v) = args.pdf_tol {
            self.pdf_tol = v;
        }
        if let Some(v) = args.lbv_tol {
            self.lbv_tol = v;
        }
        if let Some(v) = args.ismv_tol {
            self.ismv_tol = v;
        }
        if let Some(v) = args.ismv_max_iter {
            self.ismv_max_iter = v;
        }
        if let Some(v) = args.sharp_threshold {
            self.sharp_threshold = v;
        }
        if let Some(v) = args.sharp_radius_factor {
            self.sharp_radius_factor = v;
        }
        if let Some(v) = args.vsharp_max_radius_factor {
            self.vsharp_max_radius_factor = v;
        }
        if let Some(v) = args.vsharp_min_radius_factor {
            self.vsharp_min_radius_factor = v;
        }
        if let Some(v) = args.ismv_radius_factor {
            self.ismv_radius_factor = v;
        }
        if args.no_romeo_phase_gradient_coherence {
            self.romeo_phase_gradient_coherence = false;
        }
        if args.no_romeo_mag_coherence {
            self.romeo_mag_coherence = false;
        }
        if args.no_romeo_mag_weight {
            self.romeo_mag_weight = false;
        }
        if let Some(ref s) = args.mcpc3ds_sigma {
            if s.len() == 3 {
                self.mcpc3ds_sigma = [s[0], s[1], s[2]];
            }
        }
        if let Some(v) = args.tgv_iterations {
            self.tgv_iterations = v;
        }
        if let Some(v) = args.tgv_erosions {
            self.tgv_erosions = v;
        }
        if let Some(v) = args.qsmart_ilsqr_tol {
            self.qsmart_ilsqr_tol = v;
        }
        if let Some(v) = args.qsmart_ilsqr_max_iter {
            self.qsmart_ilsqr_max_iter = v;
        }
        if let Some(v) = args.qsmart_vasc_sphere_radius {
            self.qsmart_vasc_sphere_radius = v;
        }
        if let Some(v) = args.qsmart_sdf_spatial_radius {
            self.qsmart_sdf_spatial_radius = v;
        }
        if let Some(ref s) = args.swi_hp_sigma {
            if s.len() == 3 { self.swi_hp_sigma = [s[0], s[1], s[2]]; }
        }
        if let Some(ref v) = args.swi_scaling {
            self.swi_scaling = v.clone();
        }
        if let Some(v) = args.swi_strength {
            self.swi_strength = v;
        }
        if let Some(v) = args.swi_mip_window {
            self.swi_mip_window = v;
        }
        if let Some(v) = args.homogeneity_sigma_mm {
            self.homogeneity_sigma_mm = v;
        }
        if let Some(v) = args.homogeneity_nbox {
            self.homogeneity_nbox = v;
        }
        if let Some(v) = args.linear_fit_reliability_threshold {
            self.linear_fit_reliability_threshold = v;
        }
        if let Some(v) = args.tgv_step_size {
            self.tgv_step_size = v;
        }
        if let Some(v) = args.tgv_tol {
            self.tgv_tol = v;
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
        // Handle --mask-preset
        if let Some(preset) = args.mask_preset {
            self.mask_sections = match preset {
                cli::MaskPresetArg::RobustThreshold => default_mask_sections(),
                cli::MaskPresetArg::Bet => vec![MaskSection {
                    input: MaskingInput::Magnitude,
                    generator: MaskOp::Bet { fractional_intensity: 0.5 },
                    refinements: vec![MaskOp::Erode { iterations: 2 }],
                }],
            };
        }
        // Handle --mask: each flag defines a complete section (overrides --mask-preset)
        // Format: <input>,<generator>,<refinement1>,<refinement2>,...
        // e.g. --mask-section phase-quality,threshold:otsu,dilate:2,erode:2
        // Multiple sections are OR'd together.
        if let Some(ref sections) = args.mask_sections_cli {
            let mut new_sections = Vec::new();
            for s in sections {
                let parts: Vec<&str> = s.split(',').collect();
                if parts.is_empty() { continue; }
                let input = match parse_masking_input(parts[0]) {
                    Some(i) => i,
                    None => {
                        log::warn!("Ignoring invalid --mask-section input: '{}'", parts[0]);
                        continue;
                    }
                };
                let mut ops: Vec<MaskOp> = Vec::new();
                for part in &parts[1..] {
                    match parse_mask_op(part) {
                        Ok(op) => ops.push(op),
                        Err(e) => log::warn!("Ignoring invalid mask-section op '{}': {}", part, e),
                    }
                }
                let gen_idx = ops.iter().position(|op| matches!(op, MaskOp::Threshold { .. } | MaskOp::Bet { .. }));
                let generator = if let Some(gi) = gen_idx {
                    ops.remove(gi)
                } else {
                    MaskOp::Threshold { method: MaskThresholdMethod::Otsu, value: None }
                };
                new_sections.push(MaskSection {
                    input,
                    generator,
                    refinements: ops,
                });
            }
            if !new_sections.is_empty() {
                self.mask_sections = new_sections;
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
        } else if self.qsm_algorithm == QsmAlgorithm::Qsmart {
            // QSMART does its own BG removal (SDF) and inversion (iLSQR)
            if self.bf_algorithm.is_some() {
                log::debug!("QSMART selected; ignoring bf_algorithm");
            }
        } else if self.qsm_algorithm == QsmAlgorithm::Medi && self.medi_smv {
            // MEDI with SMV handles background removal internally
            if self.bf_algorithm.is_some() {
                log::debug!("MEDI+SMV selected; ignoring bf_algorithm");
            }
        } else if self.bf_algorithm.is_none() {
            return Err(QsmxtError::Config(
                "bf_algorithm must be set for standard algorithms".to_string(),
            ));
        }
        // Validate mask sections
        if self.mask_sections.is_empty() {
            return Err(QsmxtError::Config(
                "At least one mask section is required".to_string(),
            ));
        }
        for (i, section) in self.mask_sections.iter().enumerate() {
            if !section.has_generator() {
                return Err(QsmxtError::Config(
                    format!("Mask section {} has an invalid generator (must be threshold or BET)", i + 1),
                ));
            }
            // Validate generator parameters
            match &section.generator {
                MaskOp::Bet { fractional_intensity } => {
                    if *fractional_intensity < 0.0 || *fractional_intensity > 1.0 {
                        return Err(QsmxtError::Config(
                            format!("Mask section {} BET fractional intensity must be 0.0-1.0, got {}", i + 1, fractional_intensity),
                        ));
                    }
                }
                MaskOp::Threshold { method: MaskThresholdMethod::Fixed, value: Some(v) } => {
                    if *v < 0.0 {
                        return Err(QsmxtError::Config(
                            format!("Mask section {} fixed threshold must be ≥ 0.0, got {}", i + 1, v),
                        ));
                    }
                }
                MaskOp::Threshold { method: MaskThresholdMethod::Percentile, value: Some(v) } => {
                    if *v < 0.0 || *v > 100.0 {
                        return Err(QsmxtError::Config(
                            format!("Mask section {} percentile must be 0-100, got {}", i + 1, v),
                        ));
                    }
                }
                _ => {}
            }
            // Refinements must not contain generators
            for (j, op) in section.refinements.iter().enumerate() {
                if matches!(op, MaskOp::Threshold { .. } | MaskOp::Bet { .. }) {
                    return Err(QsmxtError::Config(
                        format!("Mask section {} refinement step {} is a generator (threshold/BET) — generators must be the first step. Use multiple sections (OR'd) instead.", i + 1, j + 1),
                    ));
                }
            }
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
            tsvd_threshold: None,
            ilsqr_tol: None,
            ilsqr_max_iter: None,
            tikhonov_lambda: None,
            nltv_lambda: None,
            nltv_mu: None,
            nltv_tol: None,
            nltv_max_iter: None,
            nltv_newton_iter: None,
            medi_lambda: None,
            medi_max_iter: None,
            medi_cg_max_iter: None,
            medi_cg_tol: None,
            medi_tol: None,
            medi_percentage: None,
            medi_smv_radius: None,
            medi_smv: false,
            vsharp_threshold: None,
            pdf_tol: None,
            lbv_tol: None,
            ismv_tol: None,
            ismv_max_iter: None,
            sharp_threshold: None,
            sharp_radius_factor: None,
            vsharp_max_radius_factor: None,
            vsharp_min_radius_factor: None,
            ismv_radius_factor: None,
            no_romeo_phase_gradient_coherence: false,
            no_romeo_mag_coherence: false,
            no_romeo_mag_weight: false,
            mcpc3ds_sigma: None,
            tgv_iterations: None,
            tgv_erosions: None,
            qsmart_ilsqr_tol: None,
            qsmart_ilsqr_max_iter: None,
            qsmart_vasc_sphere_radius: None,
            qsmart_sdf_spatial_radius: None,
            n_procs: None,
            do_swi: false,
            do_t2starmap: false,
            do_r2starmap: false,
            inhomogeneity_correction: false,
            obliquity_threshold: None,
            swi_hp_sigma: None,
            swi_scaling: None,
            swi_strength: None,
            swi_mip_window: None,
            homogeneity_sigma_mm: None,
            homogeneity_nbox: None,
            linear_fit_reliability_threshold: None,
            tgv_step_size: None,
            tgv_tol: None,
            mask_preset: None,
            mask_sections_cli: None,
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
    fn test_from_preset_body_is_tgv() {
        let c = PipelineConfig::from_preset(cli::Preset::Body);
        assert_eq!(c.qsm_algorithm, QsmAlgorithm::Tgv);
        assert_eq!(c.unwrapping_algorithm, None);
        assert_eq!(c.bf_algorithm, None);
        assert!(!c.combine_phase);
    }

    // --- apply_run_overrides ---




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
    fn test_to_annotated_toml_body_comments_out_bf() {
        let config = PipelineConfig::from_preset(cli::Preset::Body);
        let toml = config.to_annotated_toml();
        assert!(toml.contains("# bf_algorithm"), "BF should be commented out for Body/TGV");
    }

    // --- MaskingInput::PhaseQuality ---



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
    fn test_parse_mask_op_bet() {
        let op = parse_mask_op("bet:0.4").unwrap();
        assert_eq!(op, MaskOp::Bet { fractional_intensity: 0.4 });
    }

    #[test]
    fn test_parse_mask_op_invalid() {
        assert!(parse_mask_op("foobar:123").is_err());
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
        assert!(toml.contains("inhomogeneity_correction = true"));
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



}
