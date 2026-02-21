use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::Path;

use crate::cli;
use crate::error::QsmxtError;

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
#[serde(rename_all = "lowercase")]
pub enum MaskingInput {
    Phase,
    Magnitude,
}

impl fmt::Display for MaskingInput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Phase => write!(f, "phase"),
            Self::Magnitude => write!(f, "magnitude"),
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
    #[serde(default = "default_erosions")]
    pub mask_erosions: Vec<usize>,

    // RTS parameters
    #[serde(default = "default_rts_delta")]
    pub rts_delta: f64,
    #[serde(default = "default_rts_mu")]
    pub rts_mu: f64,
    #[serde(default = "default_rts_tol")]
    pub rts_tol: f64,

    // TV parameters
    #[serde(default = "default_tv_lambda")]
    pub tv_lambda: f64,

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
fn default_bf() -> Option<BfAlgorithm> { Some(BfAlgorithm::Pdf) }
fn default_masking() -> MaskingAlgorithm { MaskingAlgorithm::Threshold }
fn default_masking_input() -> MaskingInput { MaskingInput::Phase }
fn default_reference() -> QsmReference { QsmReference::Mean }
fn default_bet_fi() -> f64 { 0.5 }
fn default_erosions() -> Vec<usize> { vec![2] }
fn default_rts_delta() -> f64 { 0.15 }
fn default_rts_mu() -> f64 { 1e5 }
fn default_rts_tol() -> f64 { 1e-4 }
fn default_tv_lambda() -> f64 { 1e-3 }
fn default_tkd_threshold() -> f64 { 0.15 }
fn default_tgv_iterations() -> usize { 1000 }
fn default_tgv_alphas() -> [f64; 2] { [0.0015, 0.0005] }
fn default_tgv_erosions() -> usize { 3 }

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
                qsm_algorithm: QsmAlgorithm::Rts,
                unwrapping_algorithm: Some(UnwrappingAlgorithm::Romeo),
                bf_algorithm: Some(BfAlgorithm::Pdf),
                masking_algorithm: MaskingAlgorithm::Threshold,
                masking_input: MaskingInput::Phase,
                combine_phase: true,
                qsm_reference: QsmReference::Mean,
                bet_fractional_intensity: 0.5,
                mask_erosions: vec![2],
                rts_delta: 0.15,
                rts_mu: 1e5,
                rts_tol: 1e-4,
                tv_lambda: 1e-3,
                tkd_threshold: 0.15,
                tgv_iterations: 1000,
                tgv_alphas: [0.0015, 0.0005],
                tgv_erosions: 3,
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
                mask_erosions: vec![3],
                ..Self::from_preset(cli::Preset::Gre)
            },
            cli::Preset::Fast => Self {
                description: "Fast algorithms".to_string(),
                masking_algorithm: MaskingAlgorithm::Bet,
                masking_input: MaskingInput::Magnitude,
                bf_algorithm: Some(BfAlgorithm::Vsharp),
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
                masking_input: MaskingInput::Phase,
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
                cli::MaskInputArg::Phase => MaskingInput::Phase,
                cli::MaskInputArg::Magnitude => MaskingInput::Magnitude,
            };
        }
        if let Some(v) = args.combine_phase {
            self.combine_phase = v;
        }
        if let Some(v) = args.bet_fractional_intensity {
            self.bet_fractional_intensity = v;
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
        if let Some(v) = args.tv_lambda {
            self.tv_lambda = v;
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
        s.push_str(&format!("# QSM dipole inversion algorithm: rts | tv | tkd | tgv\n"));
        s.push_str(&format!("qsm_algorithm = \"{}\"\n", self.qsm_algorithm));
        s.push_str(&format!("# Phase unwrapping algorithm: romeo | laplacian\n"));
        match &self.unwrapping_algorithm {
            Some(a) => s.push_str(&format!("unwrapping_algorithm = \"{}\"\n", a)),
            None => s.push_str("# unwrapping_algorithm = \"romeo\"  # Not used with TGV\n"),
        }
        s.push_str(&format!("# Background field removal: vsharp | pdf | lbv | ismv\n"));
        match &self.bf_algorithm {
            Some(a) => s.push_str(&format!("bf_algorithm = \"{}\"\n", a)),
            None => s.push_str("# bf_algorithm = \"pdf\"  # Not used with TGV\n"),
        }
        s.push_str(&format!("# Masking method: bet | threshold\n"));
        s.push_str(&format!("masking_algorithm = \"{}\"\n", self.masking_algorithm));
        s.push_str(&format!("# Masking input: phase | magnitude\n"));
        s.push_str(&format!("masking_input = \"{}\"\n", self.masking_input));
        s.push_str(&format!("# Combine multi-echo phase data using MCPC-3D-S\n"));
        s.push_str(&format!("combine_phase = {}\n", self.combine_phase));
        s.push_str(&format!("# QSM reference: mean | none\n"));
        s.push_str(&format!("qsm_reference = \"{}\"\n", self.qsm_reference));
        s.push_str(&format!("do_swi = {}\n", self.do_swi));
        s.push('\n');

        s.push_str("[masking]\n");
        s.push_str(&format!("# BET fractional intensity (0.0-1.0, smaller = larger brain)\n"));
        s.push_str(&format!("bet_fractional_intensity = {}\n", self.bet_fractional_intensity));
        s.push_str(&format!("# Mask erosion iterations\n"));
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
