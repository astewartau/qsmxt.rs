use std::collections::HashSet;
use std::path::Path;

use crossterm::event::{KeyCode, KeyEvent};

use crate::bids::discovery::{self, BidsTree};

pub const TAB_NAMES: [&str; 4] = [
    "Input",
    "Pipeline",
    "Supplementary",
    "Execution",
];

// ─── Filter tree state ───

/// What is focused in the flattened filter tree view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterFocus {
    Pattern,
    TreeNode(usize), // index into the flattened visible node list
    NumEchoes,
}

/// A single visible row in the flattened tree (for navigation/rendering).
#[derive(Debug, Clone)]
pub enum TreeRow {
    Subject(usize),                 // index into tree.subjects
    Session(usize, usize),         // (subject_idx, session_idx)
    Run { sub: usize, ses: Option<usize>, run: usize }, // leaf run
}

/// Filter tab state: tree of BIDS runs with selection and navigation.
#[derive(Debug, Clone)]
pub struct FilterTreeState {
    pub tree: Option<BidsTree>,
    pub collapsed: HashSet<String>,
    pub focus: FilterFocus,
    pub pattern: String,
    pub pattern_editing: bool,
    pub pattern_cursor: usize,
    pub num_echoes: String,
    pub num_echoes_editing: bool,
    pub num_echoes_cursor: usize,
    pub scanned_bids_dir: Option<String>,
    pub scroll_offset: usize,
}

impl Default for FilterTreeState {
    fn default() -> Self {
        Self {
            tree: None,
            collapsed: HashSet::new(),
            focus: FilterFocus::Pattern,
            pattern: String::new(),
            pattern_editing: false,
            pattern_cursor: 0,
            num_echoes: String::new(),
            num_echoes_editing: false,
            num_echoes_cursor: 0,
            scanned_bids_dir: None,
            scroll_offset: 0,
        }
    }
}

impl FilterTreeState {
    /// Build the flattened list of visible tree rows (respecting collapsed state).
    pub fn visible_rows(&self) -> Vec<TreeRow> {
        let Some(ref tree) = self.tree else { return Vec::new() };
        let mut rows = Vec::new();
        for (si, sub) in tree.subjects.iter().enumerate() {
            rows.push(TreeRow::Subject(si));
            if self.collapsed.contains(&format!("sub-{}", sub.name)) {
                continue;
            }
            // Direct runs (no session)
            for ri in 0..sub.runs.len() {
                rows.push(TreeRow::Run { sub: si, ses: None, run: ri });
            }
            // Sessions
            for (sei, ses) in sub.sessions.iter().enumerate() {
                rows.push(TreeRow::Session(si, sei));
                if self.collapsed.contains(&format!("sub-{}/ses-{}", sub.name, ses.name)) {
                    continue;
                }
                for ri in 0..ses.runs.len() {
                    rows.push(TreeRow::Run { sub: si, ses: Some(sei), run: ri });
                }
            }
        }
        rows
    }

    /// Total number of focusable items: pattern + tree rows + num_echoes.
    fn focusable_count(&self) -> usize {
        // pattern(1) + tree rows + num_echoes(1)
        1 + self.visible_rows().len() + 1
    }

    /// Move focus down.
    pub fn focus_next(&mut self) {
        let max = self.focusable_count().saturating_sub(1);
        match self.focus {
            FilterFocus::Pattern => {
                if !self.visible_rows().is_empty() {
                    self.focus = FilterFocus::TreeNode(0);
                } else {
                    self.focus = FilterFocus::NumEchoes;
                }
            }
            FilterFocus::TreeNode(i) => {
                let rows = self.visible_rows().len();
                if i + 1 < rows {
                    self.focus = FilterFocus::TreeNode(i + 1);
                } else {
                    self.focus = FilterFocus::NumEchoes;
                }
            }
            FilterFocus::NumEchoes => {} // already at bottom
        }
        let _ = max; // used for bounds
    }

    /// Move focus up.
    pub fn focus_prev(&mut self) {
        match self.focus {
            FilterFocus::Pattern => {} // already at top
            FilterFocus::TreeNode(0) => self.focus = FilterFocus::Pattern,
            FilterFocus::TreeNode(i) => self.focus = FilterFocus::TreeNode(i - 1),
            FilterFocus::NumEchoes => {
                let rows = self.visible_rows().len();
                if rows > 0 {
                    self.focus = FilterFocus::TreeNode(rows - 1);
                } else {
                    self.focus = FilterFocus::Pattern;
                }
            }
        }
    }

    /// Scan BIDS directory if it changed since last scan.
    pub fn maybe_rescan(&mut self, bids_dir: &str) {
        let dir = bids_dir.trim().to_string();
        if dir.is_empty() {
            return;
        }
        if self.scanned_bids_dir.as_deref() == Some(&dir) {
            return;
        }
        match discovery::scan_bids_tree(Path::new(&dir)) {
            Ok(tree) => {
                self.tree = Some(tree);
                self.focus = FilterFocus::Pattern;
                self.scroll_offset = 0;
            }
            Err(_) => {
                self.tree = None;
            }
        }
        self.scanned_bids_dir = Some(dir);
    }

    /// Apply glob pattern: check runs whose display matches, uncheck others.
    pub fn apply_pattern(&mut self) {
        let Some(ref mut tree) = self.tree else { return };
        let pat = self.pattern.trim();
        if pat.is_empty() {
            // Empty pattern: select all
            tree.set_all(true);
            return;
        }
        // Convert glob to regex: escape dots, * -> .*, ? -> .
        let regex_str = format!(
            "^(?i){}$",
            pat.replace('.', r"\.")
               .replace('*', ".*")
               .replace('?', ".")
        );
        let re = match regex::Regex::new(&regex_str) {
            Ok(r) => r,
            Err(_) => return,
        };
        tree.for_each_run_mut(|run| {
            run.selected = re.is_match(&run.key_string) || re.is_match(&run.display);
        });
    }

    /// Toggle the focused tree node (run leaf or subject/session toggle).
    pub fn toggle_focused(&mut self) {
        let rows = self.visible_rows();
        let FilterFocus::TreeNode(idx) = self.focus else { return };
        let Some(row) = rows.get(idx) else { return };
        let Some(ref mut tree) = self.tree else { return };

        match row {
            TreeRow::Subject(si) => {
                let sub = &tree.subjects[*si];
                let new_val = sub.selected_runs() < sub.total_runs();
                tree.subjects[*si].set_all(new_val);
            }
            TreeRow::Session(si, sei) => {
                let ses = &tree.subjects[*si].sessions[*sei];
                let new_val = ses.runs.iter().any(|r| !r.selected);
                tree.subjects[*si].sessions[*sei].set_all(new_val);
            }
            TreeRow::Run { sub, ses, run } => {
                match ses {
                    Some(sei) => {
                        let r = &mut tree.subjects[*sub].sessions[*sei].runs[*run];
                        r.selected = !r.selected;
                    }
                    None => {
                        let r = &mut tree.subjects[*sub].runs[*run];
                        r.selected = !r.selected;
                    }
                }
            }
        }
    }

    /// Toggle collapse on the focused subject or session.
    pub fn toggle_collapse(&mut self) {
        let rows = self.visible_rows();
        let FilterFocus::TreeNode(idx) = self.focus else { return };
        let Some(row) = rows.get(idx) else { return };
        let Some(ref tree) = self.tree else { return };

        let key = match row {
            TreeRow::Subject(si) => format!("sub-{}", tree.subjects[*si].name),
            TreeRow::Session(si, sei) => format!("sub-{}/ses-{}", tree.subjects[*si].name, tree.subjects[*si].sessions[*sei].name),
            _ => return,
        };
        if self.collapsed.contains(&key) {
            self.collapsed.remove(&key);
        } else {
            self.collapsed.insert(key);
        }
    }

    /// Collect selected runs as filter values for RunArgs.
    /// Returns (subjects, sessions, acquisitions, runs) as Option<Vec<String>>.
    /// None means "all" (no filter).
    #[allow(clippy::type_complexity)]
    pub fn selected_filters(&self) -> (Option<Vec<String>>, Option<Vec<String>>, Option<Vec<String>>, Option<Vec<String>>) {
        let Some(ref tree) = self.tree else {
            return (None, None, None, None);
        };
        if tree.total_runs() == 0 || tree.selected_runs() == tree.total_runs() {
            return (None, None, None, None);
        }

        let mut subjects = HashSet::new();
        let mut sessions = HashSet::new();
        let mut acquisitions = HashSet::new();
        let mut runs = HashSet::new();

        for sub in &tree.subjects {
            for run in &sub.runs {
                if run.selected {
                    subjects.insert(format!("sub-{}", sub.name));
                    Self::extract_entities_from_key(&run.key_string, &mut sessions, &mut acquisitions, &mut runs);
                }
            }
            for ses in &sub.sessions {
                for run in &ses.runs {
                    if run.selected {
                        subjects.insert(format!("sub-{}", sub.name));
                        sessions.insert(format!("ses-{}", ses.name));
                        Self::extract_entities_from_key(&run.key_string, &mut sessions, &mut acquisitions, &mut runs);
                    }
                }
            }
        }

        // Only filter on entities that actually narrow the selection
        let to_opt = |set: HashSet<String>| -> Option<Vec<String>> {
            if set.is_empty() { None } else {
                let mut v: Vec<_> = set.into_iter().collect();
                v.sort();
                Some(v)
            }
        };

        (to_opt(subjects), None, None, None)
        // We pass subjects only — the pipeline's DiscoveryFilter will handle the rest.
        // More granular filtering would require per-run selection which the current
        // DiscoveryFilter doesn't support. Subject-level is the primary use case.
    }

    fn extract_entities_from_key(
        _key: &str,
        _sessions: &mut HashSet<String>,
        _acquisitions: &mut HashSet<String>,
        _runs: &mut HashSet<String>,
    ) {
        // Key format: sub-XX[_ses-YY][_acq-ZZ][_run-WW]_SUFFIX
        // Entity extraction is handled by the pipeline's DiscoveryFilter
    }
}

#[derive(Clone)]
pub enum FieldKind {
    Text,
    Select { options: Vec<&'static str> },
    Checkbox,
}

#[derive(Clone)]
pub struct FieldDef {
    pub label: &'static str,
    pub kind: FieldKind,
    pub help: &'static str,
}

// ─── Pipeline tab state ───

/// A visible row in the pipeline tab.
#[derive(Debug, Clone)]
pub enum PipelineRow {
    /// Algorithm selector: ◀ value ▶
    AlgoSelect {
        label: &'static str,
        field: &'static str,
        options: &'static [&'static str],
        help: &'static [&'static str], // help text per option
    },
    /// Text parameter input
    Param {
        label: &'static str,
        field: &'static str,
        help: &'static str,
    },
    /// Checkbox toggle
    Toggle {
        label: &'static str,
        field: &'static str,
        help: &'static str,
    },
    /// Section separator (blank line, not focusable)
    Separator,
    /// Section header "── Mask N ──" (not focusable)
    MaskSectionHeader { section: usize },
    /// "── OR ──" separator between sections (not focusable)
    MaskOrSeparator,
    /// Input source for a mask section (focusable, ←/→ to cycle)
    MaskOpInput { section: usize },
    /// Generator algorithm selector (threshold or BET, ←/→ to switch)
    MaskOpGenerator { section: usize },
    /// Generator parameter (threshold method or BET fractional intensity)
    MaskOpGeneratorParam { section: usize },
    /// Threshold value (only shown for fixed/percentile threshold methods)
    MaskOpThresholdValue { section: usize },
    /// A refinement step (editable, deletable, reorderable)
    MaskOpEntry { section: usize, index: usize },
    /// "Add step..." row for appending new ops to a section
    MaskOpAddStep { section: usize },
    /// "Add mask..." row for adding a new OR'd section
    MaskOpAddSection,
}

pub const MASK_OP_TYPES: &[&str] = &[
    "threshold", "bet", "erode", "dilate", "close", "fill-holes", "gaussian",
];

pub const MASK_PRESET_OPTIONS: &[&str] = &["robust-threshold", "bet", "custom"];
pub const MASK_PRESET_HELP: &[&str] = &[
    "Otsu threshold + dilate + fill holes + erode (recommended for brain)",
    "BET brain extraction + erode",
    "Fully custom mask pipeline (edit steps below)",
];

// ─── Algorithm help text (name + DOI) ───

const QSM_ALGO_HELP: &[&str] = &[
    "Rapid Two-Step (RTS) — https://doi.org/10.1016/j.neuroimage.2017.11.018",
    "Total Variation ADMM (TV) — https://doi.org/10.1002/mrm.25029",
    "Truncated K-space Division (TKD) — https://doi.org/10.1002/mrm.22135",
    "Total Generalized Variation (TGV, single-step) — https://doi.org/10.1016/j.neuroimage.2015.02.041",
    "Tikhonov L2 regularization (closed-form) — https://doi.org/10.1002/jmri.24365",
    "Nonlinear Total Variation (NLTV) — https://doi.org/10.1016/j.neuroimage.2017.11.018",
    "Morphology Enabled Dipole Inversion (MEDI) — https://doi.org/10.1002/mrm.22816",
];
const UNWRAP_HELP: &[&str] = &[
    "ROMEO region-growing unwrapping — https://doi.org/10.1002/mrm.28563",
    "Laplacian phase unwrapping (FFT-based) — https://doi.org/10.1364/OL.28.001194",
];
const BF_HELP: &[&str] = &[
    "Variable-kernel SHARP (V-SHARP) — https://doi.org/10.1002/mrm.23000",
    "Projection onto Dipole Fields (PDF) — https://doi.org/10.1002/nbm.1670",
    "Laplacian Boundary Value (LBV) — https://doi.org/10.1002/nbm.3064",
    "Iterative Spherical Mean Value (iSMV) — https://doi.org/10.1002/mrm.24998",
    "SHARP (Sophisticated Harmonic Artifact Reduction) — https://doi.org/10.1016/j.neuroimage.2010.10.070",
];
const MASK_ALGO_HELP: &[&str] = &[
    "Brain Extraction Tool (BET) — Smith 2002, https://doi.org/10.1002/hbm.10062",
    "Otsu thresholding (automatic intensity threshold)",
];
const MASK_INPUT_HELP: &[&str] = &[
    "First echo magnitude image",
    "RSS combination of all echo magnitudes",
    "Last echo magnitude image",
    "ROMEO phase quality map (spatial phase coherence)",
];
const PHASE_COMBO_HELP: &[&str] = &[
    "MCPC-3D-S: combine wrapped phase directly via phase offset estimation",
    "Linear fit: unwrap each echo, then magnitude-weighted linear fit of field vs TE",
];
const QSM_REF_HELP: &[&str] = &[
    "Subtract mean susceptibility within mask (recommended)",
    "No referencing (raw susceptibility values)",
];

/// All pipeline form values (algorithms + parameters).
#[derive(Debug, Clone)]
pub struct PipelineFormState {
    // Algorithm selections (as indices)
    pub qsm_algorithm: usize,
    pub unwrapping_algorithm: usize,
    pub bf_algorithm: usize,
    pub qsm_reference: usize,

    // Parameters (as Strings for text editing)
    pub phase_combination: usize, // 0 = mcpc3ds, 1 = linear_fit
    pub inhomogeneity_correction: bool,
    pub obliquity_threshold: String,

    // RTS
    pub rts_delta: String,
    pub rts_mu: String,
    pub rts_tol: String,
    pub rts_rho: String,
    pub rts_max_iter: String,
    pub rts_lsmr_iter: String,

    // TV
    pub tv_lambda: String,
    pub tv_rho: String,
    pub tv_tol: String,
    pub tv_max_iter: String,

    // TKD
    pub tkd_threshold: String,

    // TSVD
    pub tsvd_threshold: String,

    // iLSQR
    pub ilsqr_tol: String,
    pub ilsqr_max_iter: String,

    // TGV
    pub tgv_iterations: String,
    pub tgv_erosions: String,
    pub tgv_alpha1: String,
    pub tgv_alpha0: String,

    // Tikhonov
    pub tikhonov_lambda: String,

    // NLTV
    pub nltv_lambda: String,
    pub nltv_mu: String,
    pub nltv_tol: String,
    pub nltv_max_iter: String,
    pub nltv_newton_iter: String,

    // MEDI
    pub medi_smv: bool,
    pub medi_lambda: String,
    pub medi_max_iter: String,
    pub medi_cg_max_iter: String,
    pub medi_cg_tol: String,
    pub medi_tol: String,
    pub medi_percentage: String,
    pub medi_smv_radius: String,

    // V-SHARP
    pub vsharp_threshold: String,

    // PDF
    pub pdf_tol: String,

    // LBV
    pub lbv_tol: String,

    // iSMV
    pub ismv_tol: String,
    pub ismv_max_iter: String,

    // SHARP
    pub sharp_threshold: String,

    // QSMART
    pub qsmart_ilsqr_tol: String,
    pub qsmart_ilsqr_max_iter: String,
    pub qsmart_vasc_sphere_radius: String,
    pub qsmart_sdf_spatial_radius: String,

    // BET
    pub bet_fractional_intensity: String,
    pub bet_smoothness: String,
    pub bet_gradient_threshold: String,
    pub bet_iterations: String,
    pub bet_subdivisions: String,

    // QSM toggle
    pub do_qsm: bool,

    // ROMEO
    pub romeo_phase_gradient_coherence: bool,
    pub romeo_mag_coherence: bool,
    pub romeo_mag_weight: bool,

    // MCPC-3D-S
    pub mcpc3ds_sigma: String,

    // Mask sections (OR'd together at runtime)
    pub mask_sections: Vec<crate::pipeline::config::MaskSection>,
    pub mask_preset: usize, // 0=robust threshold, 1=BET, 2=custom

    // Pipeline tab UI state
    pub focus: usize,
    pub expanded: HashSet<String>,
    pub editing: bool,
    pub cursor: usize,
    pub scroll_offset: usize,

    // Mask ops editor state
    pub mask_ops_adding: bool,      // true when "Add step..." selector is active
    pub mask_ops_add_idx: usize,    // index into available op types during add
    pub mask_ops_add_section: usize, // which section we're adding to
    pub mask_ops_editing: Option<usize>, // index of mask op being edited (param text)
    pub mask_threshold_value_buf: String, // text buffer for editing threshold value
    pub mask_threshold_editing: bool, // true when editing threshold value
}

impl Default for PipelineFormState {
    fn default() -> Self {
        let rts = qsm_core::inversion::RtsParams::default();
        let tv = qsm_core::inversion::TvParams::default();
        let tkd = qsm_core::inversion::TkdParams::default();
        let tgv = qsm_core::inversion::TgvParams::default();
        let bet = qsm_core::bet::BetParams::default();
        Self {
            qsm_algorithm: 0, // rts
            unwrapping_algorithm: 0, // romeo
            bf_algorithm: 0, // vsharp
            qsm_reference: 0, // mean
            phase_combination: 0, // mcpc3ds
            inhomogeneity_correction: true,
            obliquity_threshold: "-1".to_string(),
            rts_delta: format!("{}", rts.delta),
            rts_mu: format!("{}", rts.mu),
            rts_tol: format!("{}", rts.tol),
            rts_rho: format!("{}", rts.rho),
            rts_max_iter: format!("{}", rts.max_iter),
            rts_lsmr_iter: format!("{}", rts.lsmr_iter),
            tv_lambda: format!("{}", tv.lambda),
            tv_rho: format!("{}", tv.rho),
            tv_tol: format!("{}", tv.tol),
            tv_max_iter: format!("{}", tv.max_iter),
            tkd_threshold: format!("{}", tkd.threshold),
            tsvd_threshold: format!("{}", tkd.threshold),
            ilsqr_tol: format!("{}", qsm_core::inversion::IlsqrParams::default().tol),
            ilsqr_max_iter: format!("{}", qsm_core::inversion::IlsqrParams::default().max_iter),
            tgv_iterations: format!("{}", tgv.iterations),
            tgv_erosions: format!("{}", tgv.erosions),
            tgv_alpha1: format!("{}", tgv.alpha1),
            tgv_alpha0: format!("{}", tgv.alpha0),
            tikhonov_lambda: format!("{}", qsm_core::inversion::TikhonovParams::default().lambda),
            nltv_lambda: format!("{}", qsm_core::inversion::NltvParams::default().lambda),
            nltv_mu: format!("{}", qsm_core::inversion::NltvParams::default().mu),
            nltv_tol: format!("{}", qsm_core::inversion::NltvParams::default().tol),
            nltv_max_iter: format!("{}", qsm_core::inversion::NltvParams::default().max_iter),
            nltv_newton_iter: format!("{}", qsm_core::inversion::NltvParams::default().newton_iter),
            medi_smv: qsm_core::inversion::MediParams::default().smv,
            medi_lambda: format!("{}", qsm_core::inversion::MediParams::default().lambda),
            medi_max_iter: format!("{}", qsm_core::inversion::MediParams::default().max_iter),
            medi_cg_max_iter: format!("{}", qsm_core::inversion::MediParams::default().cg_max_iter),
            medi_cg_tol: format!("{}", qsm_core::inversion::MediParams::default().cg_tol),
            medi_tol: format!("{}", qsm_core::inversion::MediParams::default().tol),
            medi_percentage: format!("{}", qsm_core::inversion::MediParams::default().percentage),
            medi_smv_radius: format!("{}", qsm_core::inversion::MediParams::default().smv_radius),
            vsharp_threshold: format!("{}", qsm_core::bgremove::VsharpParams::default().threshold),
            pdf_tol: format!("{}", qsm_core::bgremove::PdfParams::default().tol),
            lbv_tol: format!("{}", qsm_core::bgremove::LbvParams::default().tol),
            ismv_tol: format!("{}", qsm_core::bgremove::IsmvParams::default().tol),
            ismv_max_iter: format!("{}", qsm_core::bgremove::IsmvParams::default().max_iter),
            sharp_threshold: format!("{}", qsm_core::bgremove::SharpParams::default().threshold),
            do_qsm: true,
            romeo_phase_gradient_coherence: qsm_core::unwrap::RomeoParams::default().phase_gradient_coherence,
            romeo_mag_coherence: qsm_core::unwrap::RomeoParams::default().mag_coherence,
            romeo_mag_weight: qsm_core::unwrap::RomeoParams::default().mag_weight,
            mcpc3ds_sigma: {
                let s = qsm_core::utils::Mcpc3dsParams::default().sigma;
                format!("{} {} {}", s[0], s[1], s[2])
            },
            qsmart_ilsqr_tol: format!("{}", qsm_core::utils::QsmartParams::default().ilsqr_tol),
            qsmart_ilsqr_max_iter: format!("{}", qsm_core::utils::QsmartParams::default().ilsqr_max_iter),
            qsmart_vasc_sphere_radius: format!("{}", qsm_core::utils::QsmartParams::default().vasc_sphere_radius),
            qsmart_sdf_spatial_radius: format!("{}", qsm_core::utils::QsmartParams::default().sdf_spatial_radius),
            bet_fractional_intensity: format!("{}", bet.fractional_intensity),
            bet_smoothness: format!("{}", bet.smoothness),
            bet_gradient_threshold: format!("{}", bet.gradient_threshold),
            bet_iterations: format!("{}", bet.iterations),
            bet_subdivisions: format!("{}", bet.subdivisions),
            mask_sections: vec![crate::pipeline::config::MaskSection {
                input: crate::pipeline::config::MaskingInput::PhaseQuality,
                generator: crate::pipeline::config::MaskOp::Threshold {
                    method: crate::pipeline::config::MaskThresholdMethod::Otsu,
                    value: None,
                },
                refinements: vec![
                    crate::pipeline::config::MaskOp::Dilate { iterations: 2 },
                    crate::pipeline::config::MaskOp::FillHoles { max_size: 0 },
                    crate::pipeline::config::MaskOp::Erode { iterations: 2 },
                ],
            }],
            mask_preset: 0, // robust threshold
            focus: 0,
            expanded: HashSet::new(),
            editing: false,
            cursor: 0,
            scroll_offset: 0,
            mask_ops_adding: false,
            mask_ops_add_idx: 0,
            mask_ops_add_section: 0,
            mask_ops_editing: None,
            mask_threshold_value_buf: String::new(),
            mask_threshold_editing: false,
        }
    }
}

pub const QSM_ALGO_OPTIONS: &[&str] = &["rts", "tv", "tkd", "tsvd", "tgv", "tikhonov", "nltv", "medi", "ilsqr", "qsmart"];
pub const UNWRAP_OPTIONS: &[&str] = &["romeo", "laplacian"];
pub const BF_OPTIONS: &[&str] = &["vsharp", "pdf", "lbv", "ismv", "sharp"];
pub const MASK_ALGO_OPTIONS: &[&str] = &["bet", "threshold"];
pub const MASK_INPUT_OPTIONS: &[&str] = &["magnitude-first", "magnitude", "magnitude-last", "phase-quality"];
pub const PHASE_COMBO_OPTIONS: &[&str] = &["mcpc3ds", "linear-fit"];
pub const QSM_REF_OPTIONS: &[&str] = &["mean", "none"];

impl PipelineFormState {
    /// Build the visible rows based on current algorithm selections.
    pub fn visible_rows(&self) -> Vec<PipelineRow> {
        let mut rows = Vec::new();
        let is_tgv = self.qsm_algorithm == 4;
        let is_qsmart = self.qsm_algorithm == 9;
        let is_medi_smv = self.qsm_algorithm == 7 && self.medi_smv;

        // QSM toggle
        rows.push(PipelineRow::Toggle {
            label: "QSM Processing", field: "do_qsm",
            help: "Enable QSM reconstruction (disable to only run supplementary outputs)",
        });

        rows.push(PipelineRow::Separator);

        // General settings (QSM-only)
        if self.do_qsm {
        rows.push(PipelineRow::AlgoSelect {
            label: "Phase Combination", field: "phase_combination",
            options: PHASE_COMBO_OPTIONS, help: PHASE_COMBO_HELP,
        });
        rows.push(PipelineRow::Param {
            label: "Obliquity", field: "obliquity_threshold",
            help: "Resample oblique acquisitions to axial if obliquity exceeds this (degrees, -1 = disabled)",
        });
        rows.push(PipelineRow::Toggle {
            label: "Inhomog. Correction", field: "inhomogeneity_correction",
            help: "Apply B1 field correction to magnitude (improves masking, ROMEO weights, MEDI edges, SWI)",
        });

        rows.push(PipelineRow::Separator);
        } // end if do_qsm (general settings)

        // Mask preset selector (always visible — needed for SWI/T2*/R2* too)
        rows.push(PipelineRow::AlgoSelect {
            label: "Mask Preset", field: "mask_preset",
            options: MASK_PRESET_OPTIONS, help: MASK_PRESET_HELP,
        });

        // Mask sections
        let multi_section = self.mask_sections.len() > 1;
        for si in 0..self.mask_sections.len() {
            if si > 0 {
                rows.push(PipelineRow::MaskOrSeparator);
            }
            if multi_section {
                rows.push(PipelineRow::MaskSectionHeader { section: si });
            }
            rows.push(PipelineRow::MaskOpInput { section: si });
            rows.push(PipelineRow::MaskOpGenerator { section: si });
            rows.push(PipelineRow::MaskOpGeneratorParam { section: si });
            // Show value row for fixed/percentile threshold
            if let crate::pipeline::config::MaskOp::Threshold { method, .. } = &self.mask_sections[si].generator {
                if matches!(method, crate::pipeline::config::MaskThresholdMethod::Fixed | crate::pipeline::config::MaskThresholdMethod::Percentile) {
                    rows.push(PipelineRow::MaskOpThresholdValue { section: si });
                }
            }
            for oi in 0..self.mask_sections[si].refinements.len() {
                rows.push(PipelineRow::MaskOpEntry { section: si, index: oi });
            }
            rows.push(PipelineRow::MaskOpAddStep { section: si });
        }
        rows.push(PipelineRow::MaskOpAddSection);

        rows.push(PipelineRow::Separator);

        if self.do_qsm {
        // Unwrapping (hidden if TGV or QSMART)
        if !is_tgv && !is_qsmart {
            rows.push(PipelineRow::AlgoSelect {
                label: "Unwrapping", field: "unwrapping_algorithm",
                options: UNWRAP_OPTIONS, help: UNWRAP_HELP,
            });
            rows.push(PipelineRow::Separator);
        }

        // BG Removal (hidden for TGV, QSMART, and MEDI+SMV)
        if !is_tgv && !is_qsmart && !is_medi_smv {
            rows.push(PipelineRow::AlgoSelect {
                label: "BG Removal", field: "bf_algorithm",
                options: BF_OPTIONS, help: BF_HELP,
            });
            match self.bf_algorithm {
                0 => { // V-SHARP
                    rows.push(PipelineRow::Param { label: "  Threshold", field: "vsharp_threshold", help: "Deconvolution threshold" });
                }
                1 => { // PDF
                    rows.push(PipelineRow::Param { label: "  Tolerance", field: "pdf_tol", help: "Convergence tolerance" });
                }
                2 => { // LBV
                    rows.push(PipelineRow::Param { label: "  Tolerance", field: "lbv_tol", help: "Convergence tolerance" });
                }
                3 => { // iSMV
                    rows.push(PipelineRow::Param { label: "  Tolerance", field: "ismv_tol", help: "Convergence tolerance" });
                    rows.push(PipelineRow::Param { label: "  Max Iter", field: "ismv_max_iter", help: "Maximum iterations" });
                }
                4 => { // SHARP
                    rows.push(PipelineRow::Param { label: "  Threshold", field: "sharp_threshold", help: "Deconvolution threshold" });
                }
                _ => {}
            }
            rows.push(PipelineRow::Separator);
        }

        // QSM Inversion
        rows.push(PipelineRow::AlgoSelect {
            label: "QSM Inversion", field: "qsm_algorithm",
            options: QSM_ALGO_OPTIONS, help: QSM_ALGO_HELP,
        });

        // Algorithm-specific params
        match self.qsm_algorithm {
            0 => { // RTS
                rows.push(PipelineRow::Param { label: "  Delta", field: "rts_delta", help: "Threshold for ill-conditioned k-space region" });
                rows.push(PipelineRow::Param { label: "  Mu", field: "rts_mu", help: "Regularization parameter for well-conditioned region" });
                rows.push(PipelineRow::Param { label: "  Rho", field: "rts_rho", help: "ADMM penalty parameter" });
                rows.push(PipelineRow::Param { label: "  Tolerance", field: "rts_tol", help: "Convergence tolerance (relative change)" });
                rows.push(PipelineRow::Param { label: "  Max Iter", field: "rts_max_iter", help: "Maximum ADMM iterations" });
                rows.push(PipelineRow::Param { label: "  LSMR Iter", field: "rts_lsmr_iter", help: "LSMR iterations for step 1 (well-conditioned solve)" });
            }
            1 => { // TV
                rows.push(PipelineRow::Param { label: "  Lambda", field: "tv_lambda", help: "L1 regularization weight (smaller = smoother)" });
                rows.push(PipelineRow::Param { label: "  Rho", field: "tv_rho", help: "ADMM penalty parameter (typically 100×lambda)" });
                rows.push(PipelineRow::Param { label: "  Tolerance", field: "tv_tol", help: "Convergence tolerance" });
                rows.push(PipelineRow::Param { label: "  Max Iter", field: "tv_max_iter", help: "Maximum ADMM iterations" });
            }
            2 => { // TKD
                rows.push(PipelineRow::Param { label: "  Threshold", field: "tkd_threshold", help: "Truncation threshold for k-space division (0.1-0.2)" });
            }
            3 => { // TSVD
                rows.push(PipelineRow::Param { label: "  Threshold", field: "tsvd_threshold", help: "Truncation threshold for SVD (0.1-0.2)" });
            }
            4 => { // TGV
                rows.push(PipelineRow::Param { label: "  Iterations", field: "tgv_iterations", help: "Primal-dual iterations" });
                rows.push(PipelineRow::Param { label: "  Erosions", field: "tgv_erosions", help: "Mask erosions before TGV solve" });
                rows.push(PipelineRow::Param { label: "  Alpha1", field: "tgv_alpha1", help: "First-order TGV weight (gradient term)" });
                rows.push(PipelineRow::Param { label: "  Alpha0", field: "tgv_alpha0", help: "Second-order TGV weight (symmetric gradient term)" });
            }
            5 => { // Tikhonov
                rows.push(PipelineRow::Param { label: "  Lambda", field: "tikhonov_lambda", help: "L2 regularization weight" });
            }
            6 => { // NLTV
                rows.push(PipelineRow::Param { label: "  Lambda", field: "nltv_lambda", help: "Regularization parameter" });
                rows.push(PipelineRow::Param { label: "  Mu", field: "nltv_mu", help: "Penalty parameter" });
                rows.push(PipelineRow::Param { label: "  Tolerance", field: "nltv_tol", help: "Convergence tolerance" });
                rows.push(PipelineRow::Param { label: "  Max Iter", field: "nltv_max_iter", help: "Maximum ADMM iterations" });
                rows.push(PipelineRow::Param { label: "  Newton Iter", field: "nltv_newton_iter", help: "Newton iterations for weight update" });
            }
            7 => { // MEDI
                rows.push(PipelineRow::Toggle { label: "  SMV Mode", field: "medi_smv",
                    help: "MEDI handles background removal internally using spherical mean value preprocessing (skips the BG removal step)" });
                rows.push(PipelineRow::Param { label: "  SMV Radius", field: "medi_smv_radius", help: "SMV preprocessing radius in mm" });
                rows.push(PipelineRow::Param { label: "  Lambda", field: "medi_lambda", help: "Regularization weight" });
                rows.push(PipelineRow::Param { label: "  Percentage", field: "medi_percentage", help: "Fraction of voxels considered edges (0.0-1.0)" });
                rows.push(PipelineRow::Param { label: "  Max Iter", field: "medi_max_iter", help: "Maximum outer iterations" });
                rows.push(PipelineRow::Param { label: "  CG Max Iter", field: "medi_cg_max_iter", help: "Maximum conjugate gradient iterations" });
                rows.push(PipelineRow::Param { label: "  CG Tolerance", field: "medi_cg_tol", help: "CG convergence tolerance" });
                rows.push(PipelineRow::Param { label: "  Tolerance", field: "medi_tol", help: "Outer convergence tolerance" });
            }
            8 => { // iLSQR
                rows.push(PipelineRow::Param { label: "  Tolerance", field: "ilsqr_tol", help: "Convergence tolerance" });
                rows.push(PipelineRow::Param { label: "  Max Iter", field: "ilsqr_max_iter", help: "Maximum iterations" });
            }
            9 => { // QSMART
                rows.push(PipelineRow::Param { label: "  iLSQR Tol", field: "qsmart_ilsqr_tol", help: "iLSQR convergence tolerance" });
                rows.push(PipelineRow::Param { label: "  iLSQR Max Iter", field: "qsmart_ilsqr_max_iter", help: "Maximum iLSQR iterations per stage" });
                rows.push(PipelineRow::Param { label: "  Vasc Radius", field: "qsmart_vasc_sphere_radius", help: "Sphere radius for vasculature detection" });
                rows.push(PipelineRow::Param { label: "  SDF Radius", field: "qsmart_sdf_spatial_radius", help: "SDF spatial filtering radius" });
            }
            _ => {}
        }

        rows.push(PipelineRow::Separator);

        // QSM Reference
        rows.push(PipelineRow::AlgoSelect {
            label: "QSM Reference", field: "qsm_reference",
            options: QSM_REF_OPTIONS, help: QSM_REF_HELP,
        });
        } // end if do_qsm (unwrapping/inversion/reference)

        rows
    }

    /// Get a string parameter value by field name.
    pub fn get_param(&self, field: &str) -> &str {
        match field {
            "obliquity_threshold" => &self.obliquity_threshold,
            "rts_delta" => &self.rts_delta,
            "rts_mu" => &self.rts_mu,
            "rts_tol" => &self.rts_tol,
            "rts_rho" => &self.rts_rho,
            "rts_max_iter" => &self.rts_max_iter,
            "rts_lsmr_iter" => &self.rts_lsmr_iter,
            "tv_lambda" => &self.tv_lambda,
            "tv_rho" => &self.tv_rho,
            "tv_tol" => &self.tv_tol,
            "tv_max_iter" => &self.tv_max_iter,
            "tkd_threshold" => &self.tkd_threshold,
            "tsvd_threshold" => &self.tsvd_threshold,
            "ilsqr_tol" => &self.ilsqr_tol,
            "ilsqr_max_iter" => &self.ilsqr_max_iter,
            "tgv_iterations" => &self.tgv_iterations,
            "tgv_erosions" => &self.tgv_erosions,
            "tgv_alpha1" => &self.tgv_alpha1,
            "tgv_alpha0" => &self.tgv_alpha0,
            "tikhonov_lambda" => &self.tikhonov_lambda,
            "nltv_lambda" => &self.nltv_lambda,
            "nltv_mu" => &self.nltv_mu,
            "nltv_tol" => &self.nltv_tol,
            "nltv_max_iter" => &self.nltv_max_iter,
            "nltv_newton_iter" => &self.nltv_newton_iter,
            "medi_lambda" => &self.medi_lambda,
            "medi_max_iter" => &self.medi_max_iter,
            "medi_cg_max_iter" => &self.medi_cg_max_iter,
            "medi_cg_tol" => &self.medi_cg_tol,
            "medi_tol" => &self.medi_tol,
            "medi_percentage" => &self.medi_percentage,
            "medi_smv_radius" => &self.medi_smv_radius,
            "vsharp_threshold" => &self.vsharp_threshold,
            "pdf_tol" => &self.pdf_tol,
            "lbv_tol" => &self.lbv_tol,
            "ismv_tol" => &self.ismv_tol,
            "ismv_max_iter" => &self.ismv_max_iter,
            "sharp_threshold" => &self.sharp_threshold,
            "qsmart_ilsqr_tol" => &self.qsmart_ilsqr_tol,
            "qsmart_ilsqr_max_iter" => &self.qsmart_ilsqr_max_iter,
            "qsmart_vasc_sphere_radius" => &self.qsmart_vasc_sphere_radius,
            "qsmart_sdf_spatial_radius" => &self.qsmart_sdf_spatial_radius,
            "bet_fractional_intensity" => &self.bet_fractional_intensity,
            "bet_smoothness" => &self.bet_smoothness,
            "bet_gradient_threshold" => &self.bet_gradient_threshold,
            "bet_iterations" => &self.bet_iterations,
            "bet_subdivisions" => &self.bet_subdivisions,
            _ => "",
        }
    }

    /// Get a mutable reference to a string parameter.
    pub fn get_param_mut(&mut self, field: &str) -> Option<&mut String> {
        match field {
            "obliquity_threshold" => Some(&mut self.obliquity_threshold),
            "rts_delta" => Some(&mut self.rts_delta),
            "rts_mu" => Some(&mut self.rts_mu),
            "rts_tol" => Some(&mut self.rts_tol),
            "rts_rho" => Some(&mut self.rts_rho),
            "rts_max_iter" => Some(&mut self.rts_max_iter),
            "rts_lsmr_iter" => Some(&mut self.rts_lsmr_iter),
            "tv_lambda" => Some(&mut self.tv_lambda),
            "tv_rho" => Some(&mut self.tv_rho),
            "tv_tol" => Some(&mut self.tv_tol),
            "tv_max_iter" => Some(&mut self.tv_max_iter),
            "tkd_threshold" => Some(&mut self.tkd_threshold),
            "tsvd_threshold" => Some(&mut self.tsvd_threshold),
            "ilsqr_tol" => Some(&mut self.ilsqr_tol),
            "ilsqr_max_iter" => Some(&mut self.ilsqr_max_iter),
            "tgv_iterations" => Some(&mut self.tgv_iterations),
            "tgv_erosions" => Some(&mut self.tgv_erosions),
            "tgv_alpha1" => Some(&mut self.tgv_alpha1),
            "tgv_alpha0" => Some(&mut self.tgv_alpha0),
            "tikhonov_lambda" => Some(&mut self.tikhonov_lambda),
            "nltv_lambda" => Some(&mut self.nltv_lambda),
            "nltv_mu" => Some(&mut self.nltv_mu),
            "nltv_tol" => Some(&mut self.nltv_tol),
            "nltv_max_iter" => Some(&mut self.nltv_max_iter),
            "nltv_newton_iter" => Some(&mut self.nltv_newton_iter),
            "medi_lambda" => Some(&mut self.medi_lambda),
            "medi_max_iter" => Some(&mut self.medi_max_iter),
            "medi_cg_max_iter" => Some(&mut self.medi_cg_max_iter),
            "medi_cg_tol" => Some(&mut self.medi_cg_tol),
            "medi_tol" => Some(&mut self.medi_tol),
            "medi_percentage" => Some(&mut self.medi_percentage),
            "medi_smv_radius" => Some(&mut self.medi_smv_radius),
            "vsharp_threshold" => Some(&mut self.vsharp_threshold),
            "pdf_tol" => Some(&mut self.pdf_tol),
            "lbv_tol" => Some(&mut self.lbv_tol),
            "ismv_tol" => Some(&mut self.ismv_tol),
            "ismv_max_iter" => Some(&mut self.ismv_max_iter),
            "sharp_threshold" => Some(&mut self.sharp_threshold),
            "qsmart_ilsqr_tol" => Some(&mut self.qsmart_ilsqr_tol),
            "qsmart_ilsqr_max_iter" => Some(&mut self.qsmart_ilsqr_max_iter),
            "qsmart_vasc_sphere_radius" => Some(&mut self.qsmart_vasc_sphere_radius),
            "qsmart_sdf_spatial_radius" => Some(&mut self.qsmart_sdf_spatial_radius),
            "bet_fractional_intensity" => Some(&mut self.bet_fractional_intensity),
            "bet_smoothness" => Some(&mut self.bet_smoothness),
            "bet_gradient_threshold" => Some(&mut self.bet_gradient_threshold),
            "bet_iterations" => Some(&mut self.bet_iterations),
            "bet_subdivisions" => Some(&mut self.bet_subdivisions),
            _ => None,
        }
    }

    /// Get a select value by field name.
    pub fn get_select(&self, field: &str) -> usize {
        match field {
            "qsm_algorithm" => self.qsm_algorithm,
            "unwrapping_algorithm" => self.unwrapping_algorithm,
            "bf_algorithm" => self.bf_algorithm,
            "qsm_reference" => self.qsm_reference,
            "phase_combination" => self.phase_combination,
            "mask_preset" => self.mask_preset,
            _ => 0,
        }
    }

    /// Set a select value by field name.
    pub fn set_select(&mut self, field: &str, val: usize) {
        match field {
            "qsm_algorithm" => self.qsm_algorithm = val,
            "unwrapping_algorithm" => self.unwrapping_algorithm = val,
            "bf_algorithm" => self.bf_algorithm = val,
            "qsm_reference" => self.qsm_reference = val,
            "phase_combination" => self.phase_combination = val,
            "mask_preset" => {
                self.mask_preset = val;
                self.apply_mask_preset(val);
            }
            _ => {}
        }
    }

    /// Get a toggle value by field name.
    pub fn get_toggle(&self, field: &str) -> bool {
        match field {
            "do_qsm" => self.do_qsm,
            "inhomogeneity_correction" => self.inhomogeneity_correction,
            "medi_smv" => self.medi_smv,
            _ => false,
        }
    }

    /// Toggle a boolean by field name.
    pub fn toggle(&mut self, field: &str) {
        match field {
            "do_qsm" => self.do_qsm = !self.do_qsm,
            "inhomogeneity_correction" => self.inhomogeneity_correction = !self.inhomogeneity_correction,
            "medi_smv" => self.medi_smv = !self.medi_smv,
            _ => {}
        }
    }

    /// Get the field name of the currently focused row.
    pub fn focused_field_name(&self) -> Option<String> {
        let rows = self.visible_rows();
        let focusable = self.focusable_rows();
        let focus_idx = focusable.get(self.focus).copied()?;
        match rows.get(focus_idx) {
            Some(PipelineRow::AlgoSelect { field, .. }) => Some(field.to_string()),
            Some(PipelineRow::Param { field, .. }) => Some(field.to_string()),
            Some(PipelineRow::Toggle { field, .. }) => Some(field.to_string()),
            _ => None,
        }
    }

    /// After rows change, restore focus to the row with the given field name.
    pub fn restore_focus(&mut self, field_name: &Option<String>) {
        let Some(name) = field_name else { return };
        let rows = self.visible_rows();
        let focusable = self.focusable_rows();
        for (fi, &ri) in focusable.iter().enumerate() {
            let matches = match rows.get(ri) {
                Some(PipelineRow::AlgoSelect { field, .. }) => *field == name.as_str(),
                Some(PipelineRow::Param { field, .. }) => *field == name.as_str(),
                Some(PipelineRow::Toggle { field, .. }) => *field == name.as_str(),
                _ => false,
            };
            if matches {
                self.focus = fi;
                return;
            }
        }
        // Field not found in new layout — clamp focus
        let max = focusable.len().saturating_sub(1);
        if self.focus > max {
            self.focus = max;
        }
    }

    /// Get the display label and value for a mask op.
    pub fn mask_op_label_value(op: &crate::pipeline::config::MaskOp) -> (&'static str, String) {
        use crate::pipeline::config::MaskOp;
        match op {
            MaskOp::Threshold { method: crate::pipeline::config::MaskThresholdMethod::Otsu, .. } => ("threshold", "otsu".to_string()),
            MaskOp::Threshold { method: crate::pipeline::config::MaskThresholdMethod::Fixed, value } =>
                ("threshold", format!("fixed:{}", value.unwrap_or(0.5))),
            MaskOp::Threshold { method: crate::pipeline::config::MaskThresholdMethod::Percentile, value } =>
                ("threshold", format!("percentile:{}", value.unwrap_or(75.0))),
            MaskOp::Bet { fractional_intensity } => ("bet", format!("{}", fractional_intensity)),
            MaskOp::Erode { iterations } => ("erode", format!("{}", iterations)),
            MaskOp::Dilate { iterations } => ("dilate", format!("{}", iterations)),
            MaskOp::Close { radius } => ("close", format!("{}", radius)),
            MaskOp::FillHoles { max_size } => ("fill-holes", if *max_size == 0 { "auto".to_string() } else { format!("{}", max_size) }),
            MaskOp::GaussianSmooth { sigma_mm } => ("gaussian", format!("{}", sigma_mm)),
        }
    }

    /// Get help text for a mask op.
    pub fn mask_op_help(op: &crate::pipeline::config::MaskOp) -> &'static str {
        use crate::pipeline::config::MaskOp;
        match op {
            MaskOp::Threshold { .. } => "Threshold method (←/→ to change, Enter to edit value)",
            MaskOp::Bet { .. } => "BET fractional intensity (Enter to edit)",
            MaskOp::Erode { .. } => "Erosion iterations (←/→ to adjust)",
            MaskOp::Dilate { .. } => "Dilation iterations (←/→ to adjust)",
            MaskOp::Close { .. } => "Morphological close radius (←/→ to adjust)",
            MaskOp::FillHoles { .. } => "Fill holes max size (0=auto, Enter to edit)",
            MaskOp::GaussianSmooth { .. } => "Gaussian sigma in mm (Enter to edit)",
        }
    }

    /// Create a default mask op for the given type name.
    pub fn default_mask_op(type_name: &str) -> Option<crate::pipeline::config::MaskOp> {
        use crate::pipeline::config::*;
        match type_name {
            "threshold" => Some(MaskOp::Threshold { method: MaskThresholdMethod::Otsu, value: None }),
            "bet" => Some(MaskOp::Bet { fractional_intensity: 0.5 }),
            "erode" => Some(MaskOp::Erode { iterations: 1 }),
            "dilate" => Some(MaskOp::Dilate { iterations: 1 }),
            "close" => Some(MaskOp::Close { radius: 1 }),
            "fill-holes" => Some(MaskOp::FillHoles { max_size: 0 }),
            "gaussian" => Some(MaskOp::GaussianSmooth { sigma_mm: 4.0 }),
            _ => None,
        }
    }

    /// Apply a mask preset, overwriting mask_sections.
    pub fn apply_mask_preset(&mut self, preset: usize) {
        use crate::pipeline::config::*;
        match preset {
            0 => { // Robust threshold
                self.mask_sections = vec![MaskSection {
                    input: MaskingInput::PhaseQuality,
                    generator: MaskOp::Threshold { method: MaskThresholdMethod::Otsu, value: None },
                    refinements: vec![
                        MaskOp::Dilate { iterations: 2 },
                        MaskOp::FillHoles { max_size: 0 },
                        MaskOp::Erode { iterations: 2 },
                    ],
                }];
            }
            1 => { // BET
                self.mask_sections = vec![MaskSection {
                    input: MaskingInput::Magnitude,
                    generator: MaskOp::Bet { fractional_intensity: 0.5 },
                    refinements: vec![MaskOp::Erode { iterations: 2 }],
                }];
            }
            2 => { /* Custom: don't touch sections */ }
            _ => {}
        }
    }

    /// Mark preset as "Custom" when user manually edits mask sections.
    fn mark_mask_custom(&mut self) {
        if self.mask_preset != 2 {
            self.mask_preset = 2;
        }
    }

    /// Adjust the generator of a mask section (switch between threshold and BET).
    pub fn adjust_mask_generator(&mut self, section: usize, delta: isize) {
        use crate::pipeline::config::*;
        if section >= self.mask_sections.len() { return; }
        let gen = &self.mask_sections[section].generator;
        let new_gen = match gen {
            MaskOp::Threshold { .. } if delta > 0 => MaskOp::Bet { fractional_intensity: 0.5 },
            MaskOp::Bet { .. } if delta < 0 => MaskOp::Threshold { method: MaskThresholdMethod::Otsu, value: None },
            // Also handle wrapping
            MaskOp::Threshold { .. } => MaskOp::Bet { fractional_intensity: 0.5 },
            MaskOp::Bet { .. } => MaskOp::Threshold { method: MaskThresholdMethod::Otsu, value: None },
            _ => return,
        };
        self.mask_sections[section].generator = new_gen;
        self.mark_mask_custom();
    }

    /// Adjust the generator's parameter (threshold method or BET fractional intensity).
    pub fn adjust_mask_generator_param(&mut self, section: usize, delta: isize) {
        use crate::pipeline::config::*;
        if section >= self.mask_sections.len() { return; }
        match &mut self.mask_sections[section].generator {
            MaskOp::Threshold { method, .. } => {
                let methods = [MaskThresholdMethod::Otsu, MaskThresholdMethod::Fixed, MaskThresholdMethod::Percentile];
                let cur = methods.iter().position(|m| m == method).unwrap_or(0) as isize;
                let new = (cur + delta).rem_euclid(methods.len() as isize) as usize;
                *method = methods[new];
            }
            MaskOp::Bet { fractional_intensity } => {
                *fractional_intensity = (*fractional_intensity + delta as f64 * 0.05).clamp(0.05, 1.0);
            }
            _ => {}
        }
        self.mark_mask_custom();
    }

    /// Adjust the input source of a mask section with left/right.
    pub fn adjust_mask_input(&mut self, section: usize, delta: isize) {
        use crate::pipeline::config::MaskingInput;
        if section >= self.mask_sections.len() { return; }
        let sources = [MaskingInput::MagnitudeFirst, MaskingInput::Magnitude, MaskingInput::MagnitudeLast, MaskingInput::PhaseQuality];
        let cur = sources.iter().position(|s| *s == self.mask_sections[section].input).unwrap_or(0) as isize;
        let new = (cur + delta).rem_euclid(sources.len() as isize) as usize;
        self.mask_sections[section].input = sources[new];
        self.mark_mask_custom();
    }

    /// Adjust a mask op parameter with left/right.
    pub fn adjust_mask_op(&mut self, section: usize, index: usize, delta: isize) {
        use crate::pipeline::config::*;
        if section >= self.mask_sections.len() { return; }
        if index >= self.mask_sections[section].refinements.len() { return; }
        match &mut self.mask_sections[section].refinements[index] {
            MaskOp::Threshold { method, .. } => {
                let methods = [MaskThresholdMethod::Otsu, MaskThresholdMethod::Fixed, MaskThresholdMethod::Percentile];
                let cur = methods.iter().position(|m| m == method).unwrap_or(0) as isize;
                let new = (cur + delta).rem_euclid(methods.len() as isize) as usize;
                *method = methods[new];
            }
            MaskOp::Bet { fractional_intensity } => {
                *fractional_intensity = (*fractional_intensity + delta as f64 * 0.1).clamp(0.0, 1.0);
            }
            MaskOp::Erode { iterations } => {
                *iterations = (*iterations as isize + delta).max(1) as usize;
            }
            MaskOp::Dilate { iterations } => {
                *iterations = (*iterations as isize + delta).max(1) as usize;
            }
            MaskOp::Close { radius } => {
                *radius = (*radius as isize + delta).max(1) as usize;
            }
            MaskOp::FillHoles { max_size } => {
                *max_size = (*max_size as isize + delta * 100).max(0) as usize;
            }
            MaskOp::GaussianSmooth { sigma_mm } => {
                *sigma_mm = (*sigma_mm + delta as f64 * 0.5).max(0.5);
            }
        }
        self.mark_mask_custom();
    }

    /// Get available op types for adding refinement steps (morphological only).
    pub fn available_op_types(&self, _section: usize) -> Vec<&'static str> {
        // Generator is fixed — only offer morphological refinement ops
        MASK_OP_TYPES.iter()
            .filter(|&&t| t != "threshold" && t != "bet")
            .copied()
            .collect()
    }

    /// Get focusable row count (excludes separators and headers).
    pub fn focusable_rows(&self) -> Vec<usize> {
        self.visible_rows()
            .iter()
            .enumerate()
            .filter(|(_, r)| !matches!(r, PipelineRow::Separator | PipelineRow::MaskSectionHeader { .. } | PipelineRow::MaskOrSeparator))
            .map(|(i, _)| i)
            .collect()
    }
}

pub struct App {
    pub active_tab: usize,
    pub active_field: usize,
    pub editing: bool,
    pub cursor_pos: usize,
    pub form: RunForm,
    pub filter_state: FilterTreeState,
    pub pipeline_state: PipelineFormState,
    pub should_quit: bool,
    pub should_run: bool,
    pub tab_fields: Vec<Vec<FieldDef>>,
    pub form_scroll_offset: usize,
}

pub struct RunForm {
    // Tab 0: Input/Output
    pub bids_dir: String,
    pub output_dir: String,
    pub preset: usize,
    pub config_file: String,

    // Tab 3: Supplementary
    pub do_swi: bool,
    pub swi_scaling: usize,  // 0=tanh, 1=negative-tanh, 2=positive, 3=negative, 4=triangular
    pub swi_strength: String,
    pub swi_hp_sigma: String,
    pub swi_mip_window: String,
    pub do_t2starmap: bool,
    pub do_r2starmap: bool,

    // Tab 4: Execution
    pub dry_run: bool,
    pub debug: bool,
    pub n_procs: String,
}

impl Default for RunForm {
    fn default() -> Self {
        let swi = qsm_core::swi::SwiParams::default();
        Self {
            bids_dir: String::new(),
            output_dir: String::new(),
            preset: 0,
            config_file: String::new(),
            do_swi: false,
            swi_scaling: 0,
            swi_strength: format!("{}", swi.strength),
            swi_hp_sigma: format!("{} {} {}", swi.hp_sigma[0], swi.hp_sigma[1], swi.hp_sigma[2]),
            swi_mip_window: format!("{}", swi.mip_window),
            do_t2starmap: false,
            do_r2starmap: false,
            dry_run: false,
            debug: false,
            n_procs: String::new(),
        }
    }
}

impl App {
    pub fn new() -> Self {
        let tab_fields = vec![
            // Tab 0: Input (custom rendering — IO fields + filter tree)
            vec![],
            // Tab 1: Pipeline (custom rendering — see PipelineFormState)
            vec![],
            // Tab 2: Supplementary
            vec![
                FieldDef {
                    label: "Compute SWI",
                    kind: FieldKind::Checkbox,
                    help: "Also compute susceptibility-weighted images",
                },
                FieldDef {
                    label: "SWI Scaling",
                    kind: FieldKind::Select { options: vec!["tanh", "negative-tanh", "positive", "negative", "triangular"] },
                    help: "Phase scaling type for SWI",
                },
                FieldDef {
                    label: "SWI Strength",
                    kind: FieldKind::Text,
                    help: "Phase scaling strength (higher = stronger phase contrast)",
                },
                FieldDef {
                    label: "SWI HP Sigma",
                    kind: FieldKind::Text,
                    help: "High-pass filter sigma in voxels (X Y Z, e.g. '4 4 0'). Set Z=0 for thin axial slices.",
                },
                FieldDef {
                    label: "SWI MIP Window",
                    kind: FieldKind::Text,
                    help: "Minimum intensity projection window size in slices",
                },
                FieldDef {
                    label: "Compute T2* Map",
                    kind: FieldKind::Checkbox,
                    help: "Compute T2* relaxation map (requires 3+ echoes with magnitude)",
                },
                FieldDef {
                    label: "Compute R2* Map",
                    kind: FieldKind::Checkbox,
                    help: "Compute R2* decay rate map (requires 3+ echoes with magnitude)",
                },
            ],
            // Tab 4: Execution
            vec![
                FieldDef {
                    label: "Dry Run",
                    kind: FieldKind::Checkbox,
                    help: "Print processing plan without executing",
                },
                FieldDef {
                    label: "Debug Logging",
                    kind: FieldKind::Checkbox,
                    help: "Enable verbose debug log output",
                },
                FieldDef {
                    label: "Num Processes",
                    kind: FieldKind::Text,
                    help: "Number of parallel threads (empty = auto)",
                },
            ],
        ];

        App {
            active_tab: 0,
            active_field: 0,
            editing: false,
            cursor_pos: 0,
            form: RunForm::default(),
            filter_state: FilterTreeState::default(),
            pipeline_state: PipelineFormState::default(),
            should_quit: false,
            should_run: false,
            tab_fields,
            form_scroll_offset: 0,
        }
    }

    pub fn field_count(&self) -> usize {
        self.tab_fields[self.active_tab].len()
    }

    pub fn current_field(&self) -> &FieldDef {
        &self.tab_fields[self.active_tab][self.active_field]
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        // Route tab 0 (Input) to its combined IO + filter handler
        if self.active_tab == 0 {
            self.handle_input_tab_key(key);
            return;
        }
        // Route tab 1 (Pipeline) to its own handler
        if self.active_tab == 1 {
            self.handle_pipeline_key(key);
            return;
        }

        if self.editing {
            self.handle_editing_key(key);
            return;
        }

        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,

            // Tab switching
            KeyCode::Char(c @ '1'..='4') => {
                self.active_tab = (c as usize) - ('1' as usize);
                self.active_field = 0;
            }
            KeyCode::Tab => {
                self.active_tab = (self.active_tab + 1) % TAB_NAMES.len();
                self.active_field = 0;
            }
            KeyCode::BackTab => {
                self.active_tab = (self.active_tab + TAB_NAMES.len() - 1) % TAB_NAMES.len();
                self.active_field = 0;
            }

            // Field navigation
            KeyCode::Up | KeyCode::Char('k')
                if self.active_field > 0 => {
                    self.active_field -= 1;
                }
            KeyCode::Down | KeyCode::Char('j') => {
                let max = self.field_count().saturating_sub(1);
                if self.active_field < max {
                    self.active_field += 1;
                }
            }

            // Field interaction
            KeyCode::Enter | KeyCode::Char(' ') => self.interact_field(),
            KeyCode::Left => self.adjust_select(-1),
            KeyCode::Right => self.adjust_select(1),

            // Run
            KeyCode::F(5) => self.should_run = true,

            _ => {}
        }
    }

    // ─── Filter tab key handling ───

    /// Number of IO fields at the top of the Input tab
    pub const INPUT_IO_FIELDS: usize = 4; // bids_dir, output_dir, preset, config_file

    fn handle_input_tab_key(&mut self, key: KeyEvent) {
        // IO fields are at active_field 0-3, filter tree starts at 4+
        let in_io = self.active_field < Self::INPUT_IO_FIELDS;

        if in_io {
            // Handle IO field editing
            if self.editing {
                self.handle_editing_key(key);
                return;
            }
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
                KeyCode::Char(c @ '1'..='4') => {
                    self.active_tab = (c as usize) - ('1' as usize);
                    self.active_field = 0;
                }
                KeyCode::Tab => {
                    self.active_tab = (self.active_tab + 1) % TAB_NAMES.len();
                    self.active_field = 0;
                }
                KeyCode::BackTab => {
                    self.active_tab = (self.active_tab + TAB_NAMES.len() - 1) % TAB_NAMES.len();
                    self.active_field = 0;
                }
                KeyCode::Up | KeyCode::Char('k') if self.active_field > 0 => {
                    self.active_field -= 1;
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.active_field += 1;
                    // After IO fields, enter filter tree
                }
                KeyCode::Enter | KeyCode::Char(' ') => self.interact_io_field(),
                KeyCode::Left => self.adjust_io_select(-1),
                KeyCode::Right => self.adjust_io_select(1),
                KeyCode::F(5) => self.should_run = true,
                _ => {}
            }
            // Trigger BIDS rescan when bids_dir changes
            let bids_dir = self.form.bids_dir.clone();
            self.filter_state.maybe_rescan(&bids_dir);
        } else {
            // Delegate to filter tree handler, but intercept Up at the top to go back to IO
            if self.filter_state.pattern_editing || self.filter_state.num_echoes_editing {
                // Let filter handle its own editing
                self.handle_filter_key(key);
                return;
            }
            match key.code {
                KeyCode::Up | KeyCode::Char('k') if self.filter_state.focus == FilterFocus::Pattern => {
                    // At top of filter tree, go back to IO fields
                    self.active_field = Self::INPUT_IO_FIELDS - 1;
                }
                _ => self.handle_filter_key(key),
            }
        }
    }

    fn interact_io_field(&mut self) {
        match self.active_field {
            0 | 1 | 3 => { // Text fields: bids_dir, output_dir, config_file
                self.editing = true;
                self.cursor_pos = match self.active_field {
                    0 => self.form.bids_dir.len(),
                    1 => self.form.output_dir.len(),
                    3 => self.form.config_file.len(),
                    _ => 0,
                };
            }
            2 => { // Preset select: cycle
                self.form.preset = (self.form.preset + 1) % 6;
            }
            _ => {}
        }
    }

    fn adjust_io_select(&mut self, delta: isize) {
        if self.active_field == 2 {
            let n = 6isize; // preset options count
            self.form.preset = (self.form.preset as isize + delta).rem_euclid(n) as usize;
        }
    }

    fn handle_filter_key(&mut self, key: KeyEvent) {
        // Handle editing mode (pattern or num_echoes text input)
        if self.filter_state.pattern_editing {
            self.handle_filter_pattern_key(key);
            return;
        }
        if self.filter_state.num_echoes_editing {
            self.handle_filter_num_echoes_key(key);
            return;
        }

        // Navigation mode
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,

            // Tab switching (same as other tabs)
            KeyCode::Char(c @ '1'..='4') => {
                self.active_tab = (c as usize) - ('1' as usize);
                self.active_field = 0;
            }
            KeyCode::Tab => {
                self.active_tab = (self.active_tab + 1) % TAB_NAMES.len();
                self.active_field = 0;
            }
            KeyCode::BackTab => {
                self.active_tab = (self.active_tab + TAB_NAMES.len() - 1) % TAB_NAMES.len();
                self.active_field = 0;
            }

            // Navigation within filter tree
            KeyCode::Up | KeyCode::Char('k') => self.filter_state.focus_prev(),
            KeyCode::Down | KeyCode::Char('j') => self.filter_state.focus_next(),

            // Collapse/expand
            KeyCode::Left => self.filter_state.toggle_collapse(),
            KeyCode::Right => self.filter_state.toggle_collapse(),

            // Toggle / interact
            KeyCode::Char(' ') => self.filter_state.toggle_focused(),
            KeyCode::Enter => {
                match self.filter_state.focus {
                    FilterFocus::Pattern => {
                        self.filter_state.pattern_editing = true;
                        self.filter_state.pattern_cursor = self.filter_state.pattern.len();
                    }
                    FilterFocus::TreeNode(_) => self.filter_state.toggle_focused(),
                    FilterFocus::NumEchoes => {
                        self.filter_state.num_echoes_editing = true;
                        self.filter_state.num_echoes_cursor = self.filter_state.num_echoes.len();
                    }
                }
            }

            // Select all / none
            KeyCode::Char('a') => {
                if let Some(ref mut tree) = self.filter_state.tree {
                    tree.set_all(true);
                }
            }
            KeyCode::Char('n') => {
                if let Some(ref mut tree) = self.filter_state.tree {
                    tree.set_all(false);
                }
            }

            KeyCode::F(5) => self.should_run = true,

            _ => {}
        }
    }

    fn handle_filter_pattern_key(&mut self, key: KeyEvent) {
        let fs = &mut self.filter_state;
        match key.code {
            KeyCode::Esc => {
                fs.pattern_editing = false;
            }
            KeyCode::Enter => {
                fs.pattern_editing = false;
                fs.apply_pattern();
            }
            KeyCode::Char(c) => {
                fs.pattern.insert(fs.pattern_cursor, c);
                fs.pattern_cursor += 1;
            }
            KeyCode::Backspace if fs.pattern_cursor > 0 => {
                fs.pattern_cursor -= 1;
                fs.pattern.remove(fs.pattern_cursor);
            }
            KeyCode::Delete
                if fs.pattern_cursor < fs.pattern.len() => {
                    fs.pattern.remove(fs.pattern_cursor);
                }
            KeyCode::Left => fs.pattern_cursor = fs.pattern_cursor.saturating_sub(1),
            KeyCode::Right
                if fs.pattern_cursor < fs.pattern.len() => {
                    fs.pattern_cursor += 1;
                }
            KeyCode::Home => fs.pattern_cursor = 0,
            KeyCode::End => fs.pattern_cursor = fs.pattern.len(),
            _ => {}
        }
    }

    fn handle_filter_num_echoes_key(&mut self, key: KeyEvent) {
        let fs = &mut self.filter_state;
        match key.code {
            KeyCode::Esc | KeyCode::Enter => {
                fs.num_echoes_editing = false;
            }
            KeyCode::Char(c) => {
                fs.num_echoes.insert(fs.num_echoes_cursor, c);
                fs.num_echoes_cursor += 1;
            }
            KeyCode::Backspace if fs.num_echoes_cursor > 0 => {
                fs.num_echoes_cursor -= 1;
                fs.num_echoes.remove(fs.num_echoes_cursor);
            }
            KeyCode::Delete
                if fs.num_echoes_cursor < fs.num_echoes.len() => {
                    fs.num_echoes.remove(fs.num_echoes_cursor);
                }
            KeyCode::Left => fs.num_echoes_cursor = fs.num_echoes_cursor.saturating_sub(1),
            KeyCode::Right
                if fs.num_echoes_cursor < fs.num_echoes.len() => {
                    fs.num_echoes_cursor += 1;
                }
            KeyCode::Home => fs.num_echoes_cursor = 0,
            KeyCode::End => fs.num_echoes_cursor = fs.num_echoes.len(),
            _ => {}
        }
    }

    // ─── Pipeline tab key handling ───

    fn handle_pipeline_key(&mut self, key: KeyEvent) {
        let ps = &mut self.pipeline_state;

        if ps.editing {
            // Text editing mode for a parameter
            let rows = ps.visible_rows();
            let focusable = ps.focusable_rows();
            let focus_idx = focusable.get(ps.focus).copied().unwrap_or(0);
            if let Some(PipelineRow::Param { field, .. }) = rows.get(focus_idx) {
                let field = field.to_string();
                let mut cursor = ps.cursor;
                match key.code {
                    KeyCode::Esc | KeyCode::Enter => { ps.editing = false; return; }
                    KeyCode::Char(c) => {
                        if let Some(s) = ps.get_param_mut(&field) {
                            s.insert(cursor, c);
                            cursor += 1;
                        }
                    }
                    KeyCode::Backspace if cursor > 0 => {
                        cursor -= 1;
                        if let Some(s) = ps.get_param_mut(&field) {
                            s.remove(cursor);
                        }
                    }
                    KeyCode::Left => cursor = cursor.saturating_sub(1),
                    KeyCode::Right => {
                        let len = ps.get_param(&field).len();
                        if cursor < len { cursor += 1; }
                    }
                    KeyCode::Home => cursor = 0,
                    KeyCode::End => cursor = ps.get_param(&field).len(),
                    _ => {}
                }
                ps.cursor = cursor;
            } else {
                ps.editing = false;
            }
            return;
        }

        // Threshold value editing mode
        if ps.mask_threshold_editing {
            let mut cursor = ps.cursor;
            match key.code {
                KeyCode::Esc => {
                    ps.mask_threshold_editing = false;
                    return;
                }
                KeyCode::Enter => {
                    // Save the value back to the generator
                    let val: Option<f64> = ps.mask_threshold_value_buf.trim().parse().ok();
                    let rows = ps.visible_rows();
                    let focusable = ps.focusable_rows();
                    let focus_idx = focusable.get(ps.focus).copied().unwrap_or(0);
                    if let Some(PipelineRow::MaskOpThresholdValue { section }) = rows.get(focus_idx) {
                        if let crate::pipeline::config::MaskOp::Threshold { value, .. } = &mut ps.mask_sections[*section].generator {
                            *value = val;
                        }
                    }
                    ps.mask_threshold_editing = false;
                    ps.mark_mask_custom();
                    return;
                }
                KeyCode::Char(c) if c.is_ascii_digit() || c == '.' => {
                    ps.mask_threshold_value_buf.insert(cursor, c);
                    cursor += 1;
                }
                KeyCode::Backspace if cursor > 0 => {
                    cursor -= 1;
                    ps.mask_threshold_value_buf.remove(cursor);
                }
                KeyCode::Left => cursor = cursor.saturating_sub(1),
                KeyCode::Right if cursor < ps.mask_threshold_value_buf.len() => cursor += 1,
                _ => {}
            }
            ps.cursor = cursor;
            return;
        }

        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,

            KeyCode::Char(c @ '1'..='4') => {
                self.active_tab = (c as usize) - ('1' as usize);
                self.active_field = 0;
            }
            KeyCode::Tab => {
                self.active_tab = (self.active_tab + 1) % TAB_NAMES.len();
                self.active_field = 0;
            }
            KeyCode::BackTab => {
                self.active_tab = (self.active_tab + TAB_NAMES.len() - 1) % TAB_NAMES.len();
                self.active_field = 0;
            }

            // Reorder mask ops with Ctrl+Up/Down (must be before regular Up/Down)
            KeyCode::Up if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                let ps = &mut self.pipeline_state;
                let rows = ps.visible_rows();
                let focusable = ps.focusable_rows();
                let focus_idx = focusable.get(ps.focus).copied().unwrap_or(0);
                if let Some(PipelineRow::MaskOpEntry { section, index }) = rows.get(focus_idx) {
                    let (si, oi) = (*section, *index);
                    if oi > 0 && si < ps.mask_sections.len() && oi < ps.mask_sections[si].refinements.len() {
                        ps.mask_sections[si].refinements.swap(oi, oi - 1);
                        ps.mark_mask_custom();
                        if ps.focus > 0 { ps.focus -= 1; }
                    }
                }
            }
            KeyCode::Down if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                let ps = &mut self.pipeline_state;
                let rows = ps.visible_rows();
                let focusable = ps.focusable_rows();
                let focus_idx = focusable.get(ps.focus).copied().unwrap_or(0);
                if let Some(PipelineRow::MaskOpEntry { section, index }) = rows.get(focus_idx) {
                    let (si, oi) = (*section, *index);
                    if si < ps.mask_sections.len() && oi + 1 < ps.mask_sections[si].refinements.len() {
                        ps.mask_sections[si].refinements.swap(oi, oi + 1);
                        ps.mark_mask_custom();
                        let max = ps.focusable_rows().len().saturating_sub(1);
                        if ps.focus < max { ps.focus += 1; }
                    }
                }
            }

            // Navigation
            KeyCode::Up | KeyCode::Char('k')
                if self.pipeline_state.focus > 0 => {
                    self.pipeline_state.focus -= 1;
                }
            KeyCode::Down | KeyCode::Char('j') => {
                let max = self.pipeline_state.focusable_rows().len().saturating_sub(1);
                if self.pipeline_state.focus < max {
                    self.pipeline_state.focus += 1;
                }
            }

            // Interact
            KeyCode::Enter | KeyCode::Char(' ') => {
                let ps = &mut self.pipeline_state;
                let focused_field = ps.focused_field_name();
                let rows = ps.visible_rows();
                let focusable = ps.focusable_rows();
                let focus_idx = focusable.get(ps.focus).copied().unwrap_or(0);
                match rows.get(focus_idx).cloned() {
                    Some(PipelineRow::AlgoSelect { field, options, .. }) => {
                        let cur = ps.get_select(field);
                        ps.set_select(field, (cur + 1) % options.len());
                        ps.restore_focus(&focused_field);
                    }
                    Some(PipelineRow::Param { field, .. }) => {
                        ps.editing = true;
                        ps.cursor = ps.get_param(field).len();
                    }
                    Some(PipelineRow::Toggle { field, .. }) => {
                        let focused_field = ps.focused_field_name();
                        ps.toggle(field);
                        ps.restore_focus(&focused_field);
                    }
                    Some(PipelineRow::MaskOpAddStep { section }) => {
                        if ps.mask_ops_adding {
                            let available = ps.available_op_types(section);
                            if let Some(&type_name) = available.get(ps.mask_ops_add_idx) {
                                if let Some(op) = PipelineFormState::default_mask_op(type_name) {
                                    if section < ps.mask_sections.len() {
                                        ps.mask_sections[section].refinements.push(op);
                                    }
                                }
                            }
                            ps.mask_ops_adding = false;
                            ps.mark_mask_custom();
                        } else {
                            ps.mask_ops_adding = true;
                            ps.mask_ops_add_idx = 0;
                            ps.mask_ops_add_section = section;
                        }
                    }
                    Some(PipelineRow::MaskOpAddSection) => {
                        use crate::pipeline::config::*;
                        ps.mask_sections.push(MaskSection {
                            input: MaskingInput::Magnitude,
                            generator: MaskOp::Threshold { method: MaskThresholdMethod::Otsu, value: None },
                            refinements: vec![],
                        });
                        ps.mark_mask_custom();
                    }
                    Some(PipelineRow::MaskOpThresholdValue { section }) => {
                        // Start editing threshold value
                        let current = if let crate::pipeline::config::MaskOp::Threshold { value, .. } = &ps.mask_sections[section].generator {
                            value.map(|v| format!("{}", v)).unwrap_or_default()
                        } else { String::new() };
                        ps.mask_threshold_value_buf = current.clone();
                        ps.mask_threshold_editing = true;
                        ps.cursor = current.len();
                    }
                    Some(PipelineRow::MaskOpEntry { .. }) | Some(PipelineRow::MaskOpInput { .. })
                    | Some(PipelineRow::MaskOpGenerator { .. }) | Some(PipelineRow::MaskOpGeneratorParam { .. }) => {
                        // Handled by Left/Right
                    }
                    _ => {}
                }
            }

            // Escape from add mode
            KeyCode::Esc if self.pipeline_state.mask_ops_adding => {
                self.pipeline_state.mask_ops_adding = false;
            }

            // Delete refinement step or entire section
            KeyCode::Char('d') | KeyCode::Delete => {
                let ps = &mut self.pipeline_state;
                let rows = ps.visible_rows();
                let focusable = ps.focusable_rows();
                let focus_idx = focusable.get(ps.focus).copied().unwrap_or(0);
                match rows.get(focus_idx) {
                    Some(PipelineRow::MaskOpEntry { section, index }) => {
                        let (si, oi) = (*section, *index);
                        if si < ps.mask_sections.len() && oi < ps.mask_sections[si].refinements.len() {
                            ps.mask_sections[si].refinements.remove(oi);
                            ps.mark_mask_custom();
                            let max = ps.focusable_rows().len().saturating_sub(1);
                            if ps.focus > max { ps.focus = max; }
                        }
                    }
                    // Delete entire section (only if >1 sections) when focused on section header-adjacent rows
                    Some(PipelineRow::MaskOpGenerator { section }) | Some(PipelineRow::MaskOpInput { section }) => {
                        let si = *section;
                        if ps.mask_sections.len() > 1 && si < ps.mask_sections.len() {
                            ps.mask_sections.remove(si);
                            ps.mark_mask_custom();
                            let max = ps.focusable_rows().len().saturating_sub(1);
                            if ps.focus > max { ps.focus = max; }
                        }
                    }
                    _ => {}
                }
            }

            // Left/Right for selects and mask ops
            KeyCode::Left | KeyCode::Right => {
                let delta = if key.code == KeyCode::Left { -1isize } else { 1 };
                let ps = &mut self.pipeline_state;

                // Check if we're in mask ops add mode
                if ps.mask_ops_adding {
                    let available = ps.available_op_types(ps.mask_ops_add_section);
                    let n = available.len() as isize;
                    if n > 0 {
                        ps.mask_ops_add_idx = (ps.mask_ops_add_idx as isize + delta).rem_euclid(n) as usize;
                    }
                } else {
                    let focused_field = ps.focused_field_name();
                    let rows = ps.visible_rows();
                    let focusable = ps.focusable_rows();
                    let focus_idx = focusable.get(ps.focus).copied().unwrap_or(0);
                    match rows.get(focus_idx) {
                        Some(PipelineRow::AlgoSelect { field, options, .. }) => {
                            let n = options.len() as isize;
                            let cur = ps.get_select(field) as isize;
                            let new_val = (cur + delta).rem_euclid(n) as usize;
                            ps.set_select(field, new_val);
                            ps.restore_focus(&focused_field);
                        }
                        Some(PipelineRow::MaskOpEntry { section, index }) => {
                            ps.adjust_mask_op(*section, *index, delta);
                        }
                        Some(PipelineRow::MaskOpGenerator { section }) => {
                            ps.adjust_mask_generator(*section, delta);
                        }
                        Some(PipelineRow::MaskOpGeneratorParam { section }) => {
                            ps.adjust_mask_generator_param(*section, delta);
                        }
                        Some(PipelineRow::MaskOpInput { section }) => {
                            ps.adjust_mask_input(*section, delta);
                        }
                        _ => {}
                    }
                }
            }

            KeyCode::F(5) => self.should_run = true,

            _ => {}
        }
    }

    fn handle_editing_key(&mut self, key: KeyEvent) {
        let mut cursor = self.cursor_pos;

        match key.code {
            KeyCode::Esc | KeyCode::Enter => {
                self.editing = false;
                return;
            }
            KeyCode::Char(c) => {
                self.text_value_mut().insert(cursor, c);
                cursor += 1;
            }
            KeyCode::Backspace
                if cursor > 0 => {
                    cursor -= 1;
                    self.text_value_mut().remove(cursor);
                }
            KeyCode::Delete => {
                let len = self.text_value().len();
                if cursor < len {
                    self.text_value_mut().remove(cursor);
                }
            }
            KeyCode::Left => {
                cursor = cursor.saturating_sub(1);
            }
            KeyCode::Right => {
                let len = self.text_value().len();
                if cursor < len {
                    cursor += 1;
                }
            }
            KeyCode::Home => cursor = 0,
            KeyCode::End => cursor = self.text_value().len(),
            _ => {}
        }

        self.cursor_pos = cursor;
    }

    fn interact_field(&mut self) {
        match &self.current_field().kind {
            FieldKind::Text => {
                self.editing = true;
                self.cursor_pos = self.text_value().len();
            }
            FieldKind::Checkbox => self.toggle_checkbox(),
            FieldKind::Select { options } => {
                let n = options.len();
                let val = self.select_value();
                self.set_select_value((val + 1) % n);
            }
        }
    }

    fn adjust_select(&mut self, delta: isize) {
        if let FieldKind::Select { options } = &self.current_field().kind {
            let n = options.len() as isize;
            let val = self.select_value() as isize;
            let new_val = (val + delta).rem_euclid(n) as usize;
            self.set_select_value(new_val);
        }
    }

    // --- Field value accessors ---

    pub fn text_value(&self) -> &str {
        match (self.active_tab, self.active_field) {
            (0, 0) => &self.form.bids_dir,
            (0, 1) => &self.form.output_dir,
            (0, 3) => &self.form.config_file,
            (2, 2) => &self.form.swi_strength,
            (2, 3) => &self.form.swi_hp_sigma,
            (2, 4) => &self.form.swi_mip_window,
            (3, 2) => &self.form.n_procs,
            _ => "",
        }
    }

    fn text_value_mut(&mut self) -> &mut String {
        match (self.active_tab, self.active_field) {
            (0, 0) => &mut self.form.bids_dir,
            (0, 1) => &mut self.form.output_dir,
            (0, 3) => &mut self.form.config_file,
            (2, 2) => &mut self.form.swi_strength,
            (2, 3) => &mut self.form.swi_hp_sigma,
            (2, 4) => &mut self.form.swi_mip_window,
            (3, 2) => &mut self.form.n_procs,
            _ => unreachable!("text_value_mut called on non-text field"),
        }
    }

    pub fn select_value(&self) -> usize {
        match (self.active_tab, self.active_field) {
            (2, 1) => self.form.swi_scaling,
            _ => 0,
        }
    }

    fn set_select_value(&mut self, val: usize) {
        match (self.active_tab, self.active_field) {
            (2, 1) => self.form.swi_scaling = val,
            _ => {}
        }
    }

    #[allow(dead_code)]
    fn checkbox_value(&self) -> bool {
        match (self.active_tab, self.active_field) {
            (2, 0) => self.form.do_swi,
            (2, 5) => self.form.do_t2starmap,
            (2, 6) => self.form.do_r2starmap,
            (3, 0) => self.form.dry_run,
            (3, 1) => self.form.debug,
            _ => false,
        }
    }

    fn toggle_checkbox(&mut self) {
        match (self.active_tab, self.active_field) {
            (2, 0) => self.form.do_swi = !self.form.do_swi,
            (2, 5) => self.form.do_t2starmap = !self.form.do_t2starmap,
            (2, 6) => self.form.do_r2starmap = !self.form.do_r2starmap,
            (3, 0) => self.form.dry_run = !self.form.dry_run,
            (3, 1) => self.form.debug = !self.form.debug,
            _ => {}
        }
    }

    // Generalized accessors for rendering arbitrary (tab, field) pairs
    pub fn get_text_value(&self, tab: usize, field: usize) -> &str {
        match (tab, field) {
            (2, 2) => &self.form.swi_strength,
            (2, 3) => &self.form.swi_hp_sigma,
            (2, 4) => &self.form.swi_mip_window,
            (3, 2) => &self.form.n_procs,
            _ => "",
        }
    }

    pub fn get_select_value(&self, tab: usize, field: usize) -> usize {
        match (tab, field) {
            (2, 1) => self.form.swi_scaling,
            _ => 0,
        }
    }

    pub fn get_checkbox_value(&self, tab: usize, field: usize) -> bool {
        match (tab, field) {
            (2, 0) => self.form.do_swi,
            (2, 5) => self.form.do_t2starmap,
            (2, 6) => self.form.do_r2starmap,
            (3, 0) => self.form.dry_run,
            (3, 1) => self.form.debug,
            _ => false,
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    // --- App::new ---



    // --- Navigation ---

    #[test]
    fn test_quit_on_q() {
        let mut app = App::new();
        app.handle_key(key(KeyCode::Char('q')));
        assert!(app.should_quit);
    }

    #[test]
    fn test_quit_on_esc() {
        let mut app = App::new();
        app.handle_key(key(KeyCode::Esc));
        assert!(app.should_quit);
    }

    #[test]
    fn test_tab_switching_numbers() {
        let mut app = App::new();
        app.handle_key(key(KeyCode::Char('3')));
        assert_eq!(app.active_tab, 2);
        app.handle_key(key(KeyCode::Char('1')));
        assert_eq!(app.active_tab, 0);
        app.handle_key(key(KeyCode::Char('4')));
        assert_eq!(app.active_tab, 3);
    }



    #[test]
    fn test_tab_switch_resets_field() {
        let mut app = App::new();
        app.active_field = 2;
        app.handle_key(key(KeyCode::Tab));
        assert_eq!(app.active_field, 0);
    }

    #[test]
    fn test_field_navigation_down() {
        let mut app = App::new();
        app.handle_key(key(KeyCode::Down));
        assert_eq!(app.active_field, 1);
        app.handle_key(key(KeyCode::Char('j')));
        assert_eq!(app.active_field, 2);
    }

    #[test]
    fn test_field_navigation_up() {
        let mut app = App::new();
        app.active_field = 2;
        app.handle_key(key(KeyCode::Up));
        assert_eq!(app.active_field, 1);
        app.handle_key(key(KeyCode::Char('k')));
        assert_eq!(app.active_field, 0);
    }


    // --- Text editing ---

    #[test]
    fn test_enter_editing_text_field() {
        let mut app = App::new();
        // Tab 0, field 0 is BIDS Directory (Text)
        app.handle_key(key(KeyCode::Enter));
        assert!(app.editing);
    }

    #[test]
    fn test_type_characters() {
        let mut app = App::new();
        app.handle_key(key(KeyCode::Enter)); // enter editing
        app.handle_key(key(KeyCode::Char('/')));
        app.handle_key(key(KeyCode::Char('d')));
        app.handle_key(key(KeyCode::Char('a')));
        app.handle_key(key(KeyCode::Char('t')));
        app.handle_key(key(KeyCode::Char('a')));
        assert_eq!(app.form.bids_dir, "/data");
        assert_eq!(app.cursor_pos, 5);
    }

    #[test]
    fn test_backspace() {
        let mut app = App::new();
        app.form.bids_dir = "abc".to_string();
        app.handle_key(key(KeyCode::Enter)); // enter editing, cursor at end (3)
        app.handle_key(key(KeyCode::Backspace));
        assert_eq!(app.form.bids_dir, "ab");
        assert_eq!(app.cursor_pos, 2);
    }

    #[test]
    fn test_backspace_at_start_does_nothing() {
        let mut app = App::new();
        app.form.bids_dir = "x".to_string();
        app.editing = true;
        app.cursor_pos = 0;
        app.handle_key(key(KeyCode::Backspace));
        assert_eq!(app.form.bids_dir, "x");
    }

    #[test]
    fn test_delete_key() {
        let mut app = App::new();
        app.form.bids_dir = "abc".to_string();
        app.editing = true;
        app.cursor_pos = 0;
        app.handle_key(key(KeyCode::Delete));
        assert_eq!(app.form.bids_dir, "bc");
    }

    #[test]
    fn test_cursor_left_right() {
        let mut app = App::new();
        app.form.bids_dir = "abc".to_string();
        app.editing = true;
        app.cursor_pos = 2;
        app.handle_key(key(KeyCode::Left));
        assert_eq!(app.cursor_pos, 1);
        app.handle_key(key(KeyCode::Right));
        assert_eq!(app.cursor_pos, 2);
    }

    #[test]
    fn test_cursor_left_at_zero() {
        let mut app = App::new();
        app.editing = true;
        app.cursor_pos = 0;
        app.handle_key(key(KeyCode::Left));
        assert_eq!(app.cursor_pos, 0);
    }

    #[test]
    fn test_home_end_keys() {
        let mut app = App::new();
        app.form.bids_dir = "abcdef".to_string();
        app.editing = true;
        app.cursor_pos = 3;
        app.handle_key(key(KeyCode::Home));
        assert_eq!(app.cursor_pos, 0);
        app.handle_key(key(KeyCode::End));
        assert_eq!(app.cursor_pos, 6);
    }

    #[test]
    fn test_esc_exits_editing() {
        let mut app = App::new();
        app.editing = true;
        app.handle_key(key(KeyCode::Esc));
        assert!(!app.editing);
        // Should NOT trigger quit while editing
        assert!(!app.should_quit);
    }

    #[test]
    fn test_enter_exits_editing() {
        let mut app = App::new();
        app.editing = true;
        app.handle_key(key(KeyCode::Enter));
        assert!(!app.editing);
    }

    // --- Select fields ---

    #[test]
    fn test_select_left_right() {
        let mut app = App::new();
        // Tab 0, field 2 is Preset (Select)
        app.active_field = 2;
        assert_eq!(app.select_value(), 0);
        app.handle_key(key(KeyCode::Right));
        assert_eq!(app.form.preset, 1);
        app.handle_key(key(KeyCode::Left));
        assert_eq!(app.form.preset, 0);
    }

    #[test]
    fn test_select_wraps_around() {
        let mut app = App::new();
        app.active_field = 2; // Preset with 6 options
        app.handle_key(key(KeyCode::Left));
        assert_eq!(app.form.preset, 5); // wraps to last
    }

    #[test]
    fn test_select_enter_cycles() {
        let mut app = App::new();
        app.active_field = 2;
        app.handle_key(key(KeyCode::Enter));
        assert_eq!(app.form.preset, 1);
        // Not editing — selects don't enter edit mode
        assert!(!app.editing);
    }

    // --- Checkbox fields ---



    #[test]
    fn test_pipeline_phase_combination() {
        let mut app = App::new();
        assert_eq!(app.pipeline_state.phase_combination, 0); // mcpc3ds
        app.pipeline_state.set_select("phase_combination", 1);
        assert_eq!(app.pipeline_state.phase_combination, 1); // linear_fit
    }

    // --- F5 triggers run ---

    #[test]
    fn test_f5_triggers_run() {
        let mut app = App::new();
        app.handle_key(key(KeyCode::F(5)));
        assert!(app.should_run);
    }

    // --- Pipeline state selects ---

    #[test]
    fn test_pipeline_algorithm_selects() {
        let mut app = App::new();
        let ps = &mut app.pipeline_state;
        assert_eq!(ps.qsm_algorithm, 0); // rts
        ps.set_select("qsm_algorithm", 1);
        assert_eq!(ps.qsm_algorithm, 1); // tv
        ps.set_select("qsm_algorithm", 3);
        assert_eq!(ps.qsm_algorithm, 3); // tgv
    }

    // --- Text value accessors for different tabs ---

    #[test]
    fn test_filter_tab_routes_to_filter_handler() {
        let mut app = App::new();
        app.active_tab = 1;
        // Should not crash — filter handler takes over
        app.handle_key(key(KeyCode::Down));
        app.handle_key(key(KeyCode::Up));
        app.handle_key(key(KeyCode::Char('a')));
        app.handle_key(key(KeyCode::Char('n')));
    }

    #[test]
    fn test_pipeline_get_param() {
        let mut app = App::new();
        app.pipeline_state.rts_delta = "0.2".to_string();
        assert_eq!(app.pipeline_state.get_param("rts_delta"), "0.2");
    }


    #[test]
    fn test_text_value_unknown_returns_empty() {
        let mut app = App::new();
        app.active_tab = 2; // Algorithms tab - no text fields
        app.active_field = 0;
        assert_eq!(app.text_value(), "");
    }

    // --- get_ accessors (used by UI rendering) ---

    #[test]
    fn test_get_text_value_all_fields() {
        let app = App::new();
        // Should not panic for any valid (tab, field) combo
        for tab in 0..5 {
            for field in 0..12 {
                let _ = app.get_text_value(tab, field);
            }
        }
    }

    #[test]
    fn test_get_select_value_defaults() {
        let app = App::new();
        assert_eq!(app.get_select_value(0, 2), 0); // preset
        assert_eq!(app.get_select_value(99, 99), 0); // unknown returns 0
        // Algorithm selects are now in pipeline_state, not tab-indexed
        assert_eq!(app.pipeline_state.get_select("qsm_algorithm"), 0);
    }

    #[test]
    fn test_get_checkbox_value_defaults() {
        let app = App::new();
        assert!(!app.get_checkbox_value(4, 0)); // do_swi
        assert!(!app.get_checkbox_value(4, 4)); // dry_run
        assert!(!app.get_checkbox_value(99, 99)); // unknown returns false
    }

    // --- checkbox_value (private, exercised for coverage) ---


    // --- RunForm default ---

    #[test]
    fn test_run_form_default() {
        let form = RunForm::default();
        assert!(form.bids_dir.is_empty());
        assert!(form.output_dir.is_empty());
        assert_eq!(form.preset, 0);
        assert!(!form.do_swi);
    }


    // --- Editing output_dir ---

    #[test]
    fn test_edit_output_dir() {
        let mut app = App::new();
        app.active_field = 1; // output_dir
        app.handle_key(key(KeyCode::Enter));
        app.handle_key(key(KeyCode::Char('/')));
        app.handle_key(key(KeyCode::Char('o')));
        app.handle_key(key(KeyCode::Char('u')));
        app.handle_key(key(KeyCode::Char('t')));
        assert_eq!(app.form.output_dir, "/out");
    }

    // --- Editing config_file ---

    #[test]
    fn test_edit_config_file() {
        let mut app = App::new();
        app.active_field = 3; // config_file
        app.handle_key(key(KeyCode::Enter));
        app.handle_key(key(KeyCode::Char('c')));
        assert_eq!(app.form.config_file, "c");
    }

    // --- Editing all parameter text fields ---

    #[test]
    fn test_pipeline_param_mutation() {
        let mut ps = super::PipelineFormState::default();
        assert!(!ps.rts_delta.is_empty()); // has QSM.rs default

        // Test get_param_mut
        if let Some(s) = ps.get_param_mut("rts_delta") {
            *s = "0.25".to_string();
        }
        assert_eq!(ps.get_param("rts_delta"), "0.25");

        // Test select (phase_combination)
        ps.set_select("qsm_algorithm", 2);
        assert_eq!(ps.get_select("qsm_algorithm"), 2);
    }

    #[test]
    fn test_pipeline_visible_rows_change_with_algorithm() {
        let mut ps = super::PipelineFormState::default();
        let rows_rts = ps.visible_rows().len();
        ps.qsm_algorithm = 2; // TKD (fewer params)
        let rows_tkd = ps.visible_rows().len();
        assert!(rows_tkd < rows_rts, "TKD should have fewer rows than RTS");

        ps.qsm_algorithm = 3; // TGV (hides unwrapping + bgremove)
        let rows_tgv = ps.visible_rows().len();
        assert!(rows_tgv < rows_rts, "TGV should hide unwrapping/bgremove");
    }

    // --- Filter tree tests ---

    #[test]
    fn test_filter_state_default() {
        let fs = super::FilterTreeState::default();
        assert!(fs.tree.is_none());
        assert!(fs.pattern.is_empty());
        assert_eq!(fs.focus, super::FilterFocus::Pattern);
    }

    #[test]
    fn test_filter_scan_with_bids() {
        let dir = tempfile::tempdir().unwrap();
        crate::testutils::create_multi_echo_bids(dir.path());
        let mut fs = super::FilterTreeState::default();
        fs.maybe_rescan(dir.path().to_str().unwrap());
        assert!(fs.tree.is_some());
        let tree = fs.tree.as_ref().unwrap();
        assert_eq!(tree.subjects.len(), 1);
        assert_eq!(tree.subjects[0].name, "1");
    }

    #[test]
    fn test_filter_scan_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let mut fs = super::FilterTreeState::default();
        fs.maybe_rescan(dir.path().to_str().unwrap());
        // Tree may be Some with empty subjects, or None
        if let Some(ref tree) = fs.tree {
            assert!(tree.subjects.is_empty());
        }
    }

    #[test]
    fn test_filter_scan_caches() {
        let dir = tempfile::tempdir().unwrap();
        crate::testutils::create_single_echo_bids(dir.path());
        let mut fs = super::FilterTreeState::default();
        let path = dir.path().to_str().unwrap();
        fs.maybe_rescan(path);
        assert!(fs.tree.is_some());
        // Second call should not rescan
        fs.maybe_rescan(path);
        assert_eq!(fs.scanned_bids_dir.as_deref(), Some(path));
    }

    #[test]
    fn test_filter_navigation() {
        let dir = tempfile::tempdir().unwrap();
        crate::testutils::create_single_echo_bids(dir.path());
        let mut fs = super::FilterTreeState::default();
        fs.maybe_rescan(dir.path().to_str().unwrap());

        assert_eq!(fs.focus, super::FilterFocus::Pattern);
        fs.focus_next(); // -> tree node 0 (subject)
        assert!(matches!(fs.focus, super::FilterFocus::TreeNode(0)));
        fs.focus_next(); // -> tree node 1 (run)
        fs.focus_next(); // -> NumEchoes
        assert_eq!(fs.focus, super::FilterFocus::NumEchoes);
        fs.focus_next(); // stays at NumEchoes
        assert_eq!(fs.focus, super::FilterFocus::NumEchoes);
        fs.focus_prev(); // back up
        assert!(matches!(fs.focus, super::FilterFocus::TreeNode(_)));
    }

    #[test]
    fn test_filter_toggle_run() {
        let dir = tempfile::tempdir().unwrap();
        crate::testutils::create_single_echo_bids(dir.path());
        let mut fs = super::FilterTreeState::default();
        fs.maybe_rescan(dir.path().to_str().unwrap());

        // Navigate to the run leaf
        fs.focus_next(); // subject
        fs.focus_next(); // run leaf
        let tree = fs.tree.as_ref().unwrap();
        assert!(tree.subjects[0].runs[0].selected);
        fs.toggle_focused();
        let tree = fs.tree.as_ref().unwrap();
        assert!(!tree.subjects[0].runs[0].selected);
    }

    #[test]
    fn test_filter_select_all_none() {
        let dir = tempfile::tempdir().unwrap();
        crate::testutils::create_multi_echo_bids(dir.path());
        let mut fs = super::FilterTreeState::default();
        fs.maybe_rescan(dir.path().to_str().unwrap());

        let tree = fs.tree.as_mut().unwrap();
        tree.set_all(false);
        assert_eq!(tree.selected_runs(), 0);
        tree.set_all(true);
        assert_eq!(tree.selected_runs(), tree.total_runs());
    }



    #[test]
    fn test_filter_collapse() {
        let dir = tempfile::tempdir().unwrap();
        crate::testutils::create_single_echo_bids(dir.path());
        let mut fs = super::FilterTreeState::default();
        fs.maybe_rescan(dir.path().to_str().unwrap());

        let rows_before = fs.visible_rows().len();
        fs.focus = super::FilterFocus::TreeNode(0); // subject node
        fs.toggle_collapse();
        let rows_after = fs.visible_rows().len();
        assert!(rows_after < rows_before, "Collapsing should hide children");

        // Expand again
        fs.toggle_collapse();
        assert_eq!(fs.visible_rows().len(), rows_before);
    }

    // --- Right arrow on non-select does nothing ---

    #[test]
    fn test_left_right_on_text_field_does_nothing() {
        let mut app = App::new();
        app.active_field = 0; // Text field
        app.handle_key(key(KeyCode::Left));
        app.handle_key(key(KeyCode::Right));
        // No crash, no state change
        assert_eq!(app.active_field, 0);
    }
}
