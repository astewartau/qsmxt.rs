use std::path::PathBuf;

use crate::cli::*;
use super::app::RunForm;

pub fn build_command_string(form: &RunForm) -> String {
    let mut parts = vec!["qsmxt".to_string(), "run".to_string()];

    // Positional args
    if form.bids_dir.is_empty() {
        parts.push("<bids_dir>".to_string());
    } else {
        parts.push(form.bids_dir.clone());
    }
    if form.output_dir.is_empty() {
        parts.push("<output_dir>".to_string());
    } else {
        parts.push(form.output_dir.clone());
    }

    // Preset
    const PRESET_NAMES: [&str; 6] = ["", "gre", "epi", "bet", "fast", "body"];
    if form.preset > 0 {
        parts.push(format!("--preset {}", PRESET_NAMES[form.preset]));
    }

    // Config file
    if !form.config_file.is_empty() {
        parts.push(format!("--config {}", form.config_file));
    }

    // Filters
    if !form.subjects.is_empty() {
        parts.push(format!("--subjects {}", form.subjects));
    }
    if !form.sessions.is_empty() {
        parts.push(format!("--sessions {}", form.sessions));
    }
    if !form.acquisitions.is_empty() {
        parts.push(format!("--acquisitions {}", form.acquisitions));
    }
    if !form.runs_filter.is_empty() {
        parts.push(format!("--runs {}", form.runs_filter));
    }
    if !form.num_echoes.is_empty() {
        parts.push(format!("--num-echoes {}", form.num_echoes));
    }

    // Algorithms
    const QSM_NAMES: [&str; 4] = ["rts", "tv", "tkd", "tgv"];
    parts.push(format!("--qsm-algorithm {}", QSM_NAMES[form.qsm_algorithm]));

    const UNWRAP_NAMES: [&str; 2] = ["romeo", "laplacian"];
    parts.push(format!(
        "--unwrapping-algorithm {}",
        UNWRAP_NAMES[form.unwrapping_algorithm]
    ));

    const BF_NAMES: [&str; 4] = ["vsharp", "pdf", "lbv", "ismv"];
    parts.push(format!("--bf-algorithm {}", BF_NAMES[form.bf_algorithm]));

    const MASK_ALGO_NAMES: [&str; 2] = ["bet", "threshold"];
    parts.push(format!(
        "--masking-algorithm {}",
        MASK_ALGO_NAMES[form.masking_algorithm]
    ));

    const MASK_INPUT_NAMES: [&str; 4] = ["magnitude-first", "magnitude", "magnitude-last", "phase-quality"];
    parts.push(format!(
        "--masking-input {}",
        MASK_INPUT_NAMES[form.masking_input]
    ));

    // Parameters (only if non-empty)
    if form.combine_phase {
        parts.push("--combine-phase true".to_string());
    }
    push_if_set(&mut parts, "--bet-fractional-intensity", &form.bet_fractional_intensity);
    push_if_set(&mut parts, "--mask-erosions", &form.mask_erosions);
    push_if_set(&mut parts, "--rts-delta", &form.rts_delta);
    push_if_set(&mut parts, "--rts-mu", &form.rts_mu);
    push_if_set(&mut parts, "--rts-tol", &form.rts_tol);
    push_if_set(&mut parts, "--tgv-iterations", &form.tgv_iterations);
    push_if_set(&mut parts, "--tgv-erosions", &form.tgv_erosions);
    push_if_set(&mut parts, "--tv-lambda", &form.tv_lambda);
    push_if_set(&mut parts, "--tkd-threshold", &form.tkd_threshold);
    push_if_set(&mut parts, "--obliquity-threshold", &form.obliquity_threshold);

    // Mask ops (if any, overrides legacy masking)
    for op in &form.mask_ops {
        parts.push(format!("--mask-op {}", op));
    }

    // Execution flags
    if form.do_swi {
        parts.push("--do-swi".to_string());
    }
    if form.do_t2starmap {
        parts.push("--do-t2starmap".to_string());
    }
    if form.do_r2starmap {
        parts.push("--do-r2starmap".to_string());
    }
    if form.inhomogeneity_correction {
        parts.push("--inhomogeneity-correction".to_string());
    }
    if form.dry_run {
        parts.push("--dry".to_string());
    }
    if form.debug {
        parts.push("--debug".to_string());
    }
    push_if_set(&mut parts, "--n-procs", &form.n_procs);

    parts.join(" ")
}

fn push_if_set(parts: &mut Vec<String>, flag: &str, value: &str) {
    let trimmed = value.trim();
    if !trimmed.is_empty() {
        parts.push(format!("{} {}", flag, trimmed));
    }
}

pub fn build_run_args(form: &RunForm) -> crate::Result<RunArgs> {
    if form.bids_dir.is_empty() || form.output_dir.is_empty() {
        return Err(crate::error::QsmxtError::Config(
            "BIDS directory and output directory are required".to_string(),
        ));
    }

    let preset_options = [
        None,
        Some(Preset::Gre),
        Some(Preset::Epi),
        Some(Preset::Bet),
        Some(Preset::Fast),
        Some(Preset::Body),
    ];
    let qsm_options = [
        QsmAlgorithmArg::Rts,
        QsmAlgorithmArg::Tv,
        QsmAlgorithmArg::Tkd,
        QsmAlgorithmArg::Tgv,
    ];
    let unwrap_options = [UnwrapAlgorithmArg::Romeo, UnwrapAlgorithmArg::Laplacian];
    let bf_options = [
        BfAlgorithmArg::Vsharp,
        BfAlgorithmArg::Pdf,
        BfAlgorithmArg::Lbv,
        BfAlgorithmArg::Ismv,
    ];
    let mask_algo_options = [MaskAlgorithmArg::Bet, MaskAlgorithmArg::Threshold];
    let mask_input_options = [MaskInputArg::MagnitudeFirst, MaskInputArg::Magnitude, MaskInputArg::MagnitudeLast, MaskInputArg::PhaseQuality];

    Ok(RunArgs {
        bids_dir: PathBuf::from(&form.bids_dir),
        output_dir: PathBuf::from(&form.output_dir),
        preset: preset_options[form.preset],
        config: parse_optional_path(&form.config_file),
        subjects: parse_optional_string_vec(&form.subjects),
        sessions: parse_optional_string_vec(&form.sessions),
        acquisitions: parse_optional_string_vec(&form.acquisitions),
        runs: parse_optional_string_vec(&form.runs_filter),
        num_echoes: parse_optional_usize(&form.num_echoes),
        qsm_algorithm: Some(qsm_options[form.qsm_algorithm]),
        unwrapping_algorithm: Some(unwrap_options[form.unwrapping_algorithm]),
        bf_algorithm: Some(bf_options[form.bf_algorithm]),
        masking_algorithm: Some(mask_algo_options[form.masking_algorithm]),
        masking_input: Some(mask_input_options[form.masking_input]),
        combine_phase: if form.combine_phase { Some(true) } else { None },
        bet_fractional_intensity: parse_optional_f64(&form.bet_fractional_intensity),
        mask_erosions: parse_optional_usize_vec(&form.mask_erosions),
        rts_delta: parse_optional_f64(&form.rts_delta),
        rts_mu: parse_optional_f64(&form.rts_mu),
        rts_tol: parse_optional_f64(&form.rts_tol),
        tgv_iterations: parse_optional_usize(&form.tgv_iterations),
        tgv_erosions: parse_optional_usize(&form.tgv_erosions),
        tv_lambda: parse_optional_f64(&form.tv_lambda),
        tkd_threshold: parse_optional_f64(&form.tkd_threshold),
        n_procs: parse_optional_usize(&form.n_procs),
        do_swi: form.do_swi,
        do_t2starmap: form.do_t2starmap,
        do_r2starmap: form.do_r2starmap,
        inhomogeneity_correction: form.inhomogeneity_correction,
        obliquity_threshold: parse_optional_f64(&form.obliquity_threshold),
        mask_ops: if form.mask_ops.is_empty() { None } else {
            Some(form.mask_ops.iter().map(|op| format!("{}", op)).collect())
        },
        dry: form.dry_run,
        debug: form.debug,
        mem_limit_gb: None,
        no_mem_limit: false,
        force: false,
        clean_intermediates: false,
    })
}

fn parse_optional_path(s: &str) -> Option<PathBuf> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(PathBuf::from(trimmed))
    }
}

fn parse_optional_string_vec(s: &str) -> Option<Vec<String>> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.split_whitespace().map(String::from).collect())
    }
}

fn parse_optional_f64(s: &str) -> Option<f64> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        None
    } else {
        trimmed.parse().ok()
    }
}

fn parse_optional_usize(s: &str) -> Option<usize> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        None
    } else {
        trimmed.parse().ok()
    }
}

fn parse_optional_usize_vec(s: &str) -> Option<Vec<usize>> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        None
    } else {
        let parsed: Vec<usize> = trimmed
            .split_whitespace()
            .filter_map(|w| w.parse().ok())
            .collect();
        if parsed.is_empty() { None } else { Some(parsed) }
    }
}
