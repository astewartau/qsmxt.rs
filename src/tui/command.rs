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

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::app::RunForm;

    fn default_form() -> RunForm {
        RunForm::default()
    }

    // --- build_command_string ---

    #[test]
    fn test_command_string_minimal() {
        let form = default_form();
        let cmd = build_command_string(&form);
        assert!(cmd.starts_with("qsmxt run"));
        assert!(cmd.contains("<bids_dir>"));
        assert!(cmd.contains("<output_dir>"));
    }

    #[test]
    fn test_command_string_with_dirs() {
        let mut form = default_form();
        form.bids_dir = "/data/bids".to_string();
        form.output_dir = "/data/out".to_string();
        let cmd = build_command_string(&form);
        assert!(cmd.contains("/data/bids"));
        assert!(cmd.contains("/data/out"));
        assert!(!cmd.contains("<bids_dir>"));
    }

    #[test]
    fn test_command_string_with_preset() {
        let mut form = default_form();
        form.bids_dir = "/b".to_string();
        form.output_dir = "/o".to_string();
        form.preset = 1; // gre
        let cmd = build_command_string(&form);
        assert!(cmd.contains("--preset gre"));
    }

    #[test]
    fn test_command_string_no_preset_when_zero() {
        let form = default_form();
        let cmd = build_command_string(&form);
        assert!(!cmd.contains("--preset"));
    }

    #[test]
    fn test_command_string_with_config() {
        let mut form = default_form();
        form.config_file = "my.toml".to_string();
        let cmd = build_command_string(&form);
        assert!(cmd.contains("--config my.toml"));
    }

    #[test]
    fn test_command_string_with_filters() {
        let mut form = default_form();
        form.subjects = "sub-01 sub-02".to_string();
        form.sessions = "ses-pre".to_string();
        form.acquisitions = "acq-gre".to_string();
        form.runs_filter = "1".to_string();
        form.num_echoes = "4".to_string();
        let cmd = build_command_string(&form);
        assert!(cmd.contains("--subjects sub-01 sub-02"));
        assert!(cmd.contains("--sessions ses-pre"));
        assert!(cmd.contains("--acquisitions acq-gre"));
        assert!(cmd.contains("--runs 1"));
        assert!(cmd.contains("--num-echoes 4"));
    }

    #[test]
    fn test_command_string_algorithms() {
        let mut form = default_form();
        form.qsm_algorithm = 2; // tkd
        form.unwrapping_algorithm = 1; // laplacian
        form.bf_algorithm = 3; // ismv
        form.masking_algorithm = 0; // bet
        form.masking_input = 3; // phase-quality
        let cmd = build_command_string(&form);
        assert!(cmd.contains("--qsm-algorithm tkd"));
        assert!(cmd.contains("--unwrapping-algorithm laplacian"));
        assert!(cmd.contains("--bf-algorithm ismv"));
        assert!(cmd.contains("--masking-algorithm bet"));
        assert!(cmd.contains("--masking-input phase-quality"));
    }

    #[test]
    fn test_command_string_parameters() {
        let mut form = default_form();
        form.bet_fractional_intensity = "0.3".to_string();
        form.rts_delta = "0.2".to_string();
        form.obliquity_threshold = "5".to_string();
        let cmd = build_command_string(&form);
        assert!(cmd.contains("--bet-fractional-intensity 0.3"));
        assert!(cmd.contains("--rts-delta 0.2"));
        assert!(cmd.contains("--obliquity-threshold 5"));
    }

    #[test]
    fn test_command_string_empty_params_omitted() {
        let form = default_form();
        let cmd = build_command_string(&form);
        assert!(!cmd.contains("--bet-fractional-intensity"));
        assert!(!cmd.contains("--rts-delta"));
        assert!(!cmd.contains("--n-procs"));
    }

    #[test]
    fn test_command_string_combine_phase() {
        let mut form = default_form();
        form.combine_phase = true;
        let cmd = build_command_string(&form);
        assert!(cmd.contains("--combine-phase true"));
    }

    #[test]
    fn test_command_string_execution_flags() {
        let mut form = default_form();
        form.do_swi = true;
        form.do_t2starmap = true;
        form.do_r2starmap = true;
        form.inhomogeneity_correction = true;
        form.dry_run = true;
        form.debug = true;
        form.n_procs = "4".to_string();
        let cmd = build_command_string(&form);
        assert!(cmd.contains("--do-swi"));
        assert!(cmd.contains("--do-t2starmap"));
        assert!(cmd.contains("--do-r2starmap"));
        assert!(cmd.contains("--inhomogeneity-correction"));
        assert!(cmd.contains("--dry"));
        assert!(cmd.contains("--debug"));
        assert!(cmd.contains("--n-procs 4"));
    }

    #[test]
    fn test_command_string_mask_ops() {
        let mut form = default_form();
        form.mask_ops = vec![
            crate::pipeline::config::MaskOp::Erode { iterations: 2 },
            crate::pipeline::config::MaskOp::Dilate { iterations: 1 },
        ];
        let cmd = build_command_string(&form);
        assert!(cmd.contains("--mask-op erode:2"));
        assert!(cmd.contains("--mask-op dilate:1"));
    }

    // --- build_run_args ---

    #[test]
    fn test_build_run_args_error_when_empty() {
        let form = default_form();
        let result = build_run_args(&form);
        assert!(result.is_err());
    }

    #[test]
    fn test_build_run_args_minimal() {
        let mut form = default_form();
        form.bids_dir = "/bids".to_string();
        form.output_dir = "/out".to_string();
        let args = build_run_args(&form).unwrap();
        assert_eq!(args.bids_dir, PathBuf::from("/bids"));
        assert_eq!(args.output_dir, PathBuf::from("/out"));
        assert_eq!(args.preset, None); // preset 0 = None
        assert_eq!(args.qsm_algorithm, Some(crate::cli::QsmAlgorithmArg::Rts));
    }

    #[test]
    fn test_build_run_args_with_preset() {
        let mut form = default_form();
        form.bids_dir = "/b".to_string();
        form.output_dir = "/o".to_string();
        form.preset = 2; // epi
        let args = build_run_args(&form).unwrap();
        assert_eq!(args.preset, Some(crate::cli::Preset::Epi));
    }

    #[test]
    fn test_build_run_args_all_presets() {
        for (idx, expected) in [
            (0, None),
            (1, Some(crate::cli::Preset::Gre)),
            (2, Some(crate::cli::Preset::Epi)),
            (3, Some(crate::cli::Preset::Bet)),
            (4, Some(crate::cli::Preset::Fast)),
            (5, Some(crate::cli::Preset::Body)),
        ] {
            let mut form = default_form();
            form.bids_dir = "/b".to_string();
            form.output_dir = "/o".to_string();
            form.preset = idx;
            let args = build_run_args(&form).unwrap();
            assert_eq!(args.preset, expected);
        }
    }

    #[test]
    fn test_build_run_args_algorithms() {
        let mut form = default_form();
        form.bids_dir = "/b".to_string();
        form.output_dir = "/o".to_string();
        form.qsm_algorithm = 3; // tgv
        form.unwrapping_algorithm = 1; // laplacian
        form.bf_algorithm = 2; // lbv
        form.masking_algorithm = 0; // bet
        form.masking_input = 2; // magnitude-last
        let args = build_run_args(&form).unwrap();
        assert_eq!(args.qsm_algorithm, Some(crate::cli::QsmAlgorithmArg::Tgv));
        assert_eq!(args.unwrapping_algorithm, Some(crate::cli::UnwrapAlgorithmArg::Laplacian));
        assert_eq!(args.bf_algorithm, Some(crate::cli::BfAlgorithmArg::Lbv));
        assert_eq!(args.masking_algorithm, Some(crate::cli::MaskAlgorithmArg::Bet));
        assert_eq!(args.masking_input, Some(crate::cli::MaskInputArg::MagnitudeLast));
    }

    #[test]
    fn test_build_run_args_filters() {
        let mut form = default_form();
        form.bids_dir = "/b".to_string();
        form.output_dir = "/o".to_string();
        form.subjects = "sub-01 sub-02".to_string();
        form.sessions = "ses-pre".to_string();
        form.num_echoes = "4".to_string();
        let args = build_run_args(&form).unwrap();
        assert_eq!(args.subjects, Some(vec!["sub-01".to_string(), "sub-02".to_string()]));
        assert_eq!(args.sessions, Some(vec!["ses-pre".to_string()]));
        assert_eq!(args.num_echoes, Some(4));
    }

    #[test]
    fn test_build_run_args_numeric_params() {
        let mut form = default_form();
        form.bids_dir = "/b".to_string();
        form.output_dir = "/o".to_string();
        form.bet_fractional_intensity = "0.3".to_string();
        form.rts_delta = "0.2".to_string();
        form.rts_mu = "1e5".to_string();
        form.rts_tol = "1e-4".to_string();
        form.tgv_iterations = "500".to_string();
        form.tgv_erosions = "2".to_string();
        form.tv_lambda = "0.001".to_string();
        form.tkd_threshold = "0.15".to_string();
        form.n_procs = "8".to_string();
        let args = build_run_args(&form).unwrap();
        assert_eq!(args.bet_fractional_intensity, Some(0.3));
        assert_eq!(args.rts_delta, Some(0.2));
        assert_eq!(args.tgv_iterations, Some(500));
        assert_eq!(args.n_procs, Some(8));
    }

    #[test]
    fn test_build_run_args_flags() {
        let mut form = default_form();
        form.bids_dir = "/b".to_string();
        form.output_dir = "/o".to_string();
        form.do_swi = true;
        form.do_t2starmap = true;
        form.do_r2starmap = true;
        form.inhomogeneity_correction = true;
        form.dry_run = true;
        form.debug = true;
        form.combine_phase = true;
        let args = build_run_args(&form).unwrap();
        assert!(args.do_swi);
        assert!(args.do_t2starmap);
        assert!(args.do_r2starmap);
        assert!(args.inhomogeneity_correction);
        assert!(args.dry);
        assert!(args.debug);
        assert_eq!(args.combine_phase, Some(true));
    }

    #[test]
    fn test_build_run_args_combine_phase_false() {
        let mut form = default_form();
        form.bids_dir = "/b".to_string();
        form.output_dir = "/o".to_string();
        form.combine_phase = false;
        let args = build_run_args(&form).unwrap();
        assert_eq!(args.combine_phase, None);
    }

    #[test]
    fn test_build_run_args_mask_ops() {
        let mut form = default_form();
        form.bids_dir = "/b".to_string();
        form.output_dir = "/o".to_string();
        form.mask_ops = vec![
            crate::pipeline::config::MaskOp::Erode { iterations: 2 },
        ];
        let args = build_run_args(&form).unwrap();
        assert!(args.mask_ops.is_some());
        assert_eq!(args.mask_ops.as_ref().unwrap().len(), 1);
    }

    #[test]
    fn test_build_run_args_empty_mask_ops() {
        let mut form = default_form();
        form.bids_dir = "/b".to_string();
        form.output_dir = "/o".to_string();
        let args = build_run_args(&form).unwrap();
        assert!(args.mask_ops.is_none());
    }

    #[test]
    fn test_build_run_args_config_file() {
        let mut form = default_form();
        form.bids_dir = "/b".to_string();
        form.output_dir = "/o".to_string();
        form.config_file = "pipeline.toml".to_string();
        let args = build_run_args(&form).unwrap();
        assert_eq!(args.config, Some(PathBuf::from("pipeline.toml")));
    }

    #[test]
    fn test_build_run_args_mask_erosions() {
        let mut form = default_form();
        form.bids_dir = "/b".to_string();
        form.output_dir = "/o".to_string();
        form.mask_erosions = "2 3 4".to_string();
        let args = build_run_args(&form).unwrap();
        assert_eq!(args.mask_erosions, Some(vec![2, 3, 4]));
    }

    #[test]
    fn test_build_run_args_obliquity() {
        let mut form = default_form();
        form.bids_dir = "/b".to_string();
        form.output_dir = "/o".to_string();
        form.obliquity_threshold = "5.0".to_string();
        let args = build_run_args(&form).unwrap();
        assert_eq!(args.obliquity_threshold, Some(5.0));
    }

    // --- parse helpers ---

    #[test]
    fn test_parse_optional_path_empty() {
        assert_eq!(parse_optional_path(""), None);
        assert_eq!(parse_optional_path("  "), None);
    }

    #[test]
    fn test_parse_optional_path_value() {
        assert_eq!(parse_optional_path("/foo"), Some(PathBuf::from("/foo")));
        assert_eq!(parse_optional_path("  /bar  "), Some(PathBuf::from("/bar")));
    }

    #[test]
    fn test_parse_optional_f64_empty() {
        assert_eq!(parse_optional_f64(""), None);
        assert_eq!(parse_optional_f64("  "), None);
    }

    #[test]
    fn test_parse_optional_f64_valid() {
        assert_eq!(parse_optional_f64("2.72"), Some(2.72));
        assert_eq!(parse_optional_f64("  1e-4  "), Some(1e-4));
    }

    #[test]
    fn test_parse_optional_f64_invalid() {
        assert_eq!(parse_optional_f64("abc"), None);
    }

    #[test]
    fn test_parse_optional_usize_empty() {
        assert_eq!(parse_optional_usize(""), None);
    }

    #[test]
    fn test_parse_optional_usize_valid() {
        assert_eq!(parse_optional_usize("42"), Some(42));
    }

    #[test]
    fn test_parse_optional_usize_invalid() {
        assert_eq!(parse_optional_usize("abc"), None);
    }

    #[test]
    fn test_parse_optional_string_vec_empty() {
        assert_eq!(parse_optional_string_vec(""), None);
        assert_eq!(parse_optional_string_vec("   "), None);
    }

    #[test]
    fn test_parse_optional_string_vec_values() {
        assert_eq!(
            parse_optional_string_vec("a b c"),
            Some(vec!["a".to_string(), "b".to_string(), "c".to_string()])
        );
    }

    #[test]
    fn test_parse_optional_usize_vec_empty() {
        assert_eq!(parse_optional_usize_vec(""), None);
    }

    #[test]
    fn test_parse_optional_usize_vec_valid() {
        assert_eq!(parse_optional_usize_vec("1 2 3"), Some(vec![1, 2, 3]));
    }

    #[test]
    fn test_parse_optional_usize_vec_invalid_returns_none() {
        assert_eq!(parse_optional_usize_vec("abc def"), None);
    }

    #[test]
    fn test_parse_optional_usize_vec_mixed() {
        // Only valid values are kept
        assert_eq!(parse_optional_usize_vec("1 abc 3"), Some(vec![1, 3]));
    }

    // --- push_if_set ---

    #[test]
    fn test_push_if_set_empty() {
        let mut parts = vec![];
        push_if_set(&mut parts, "--flag", "");
        push_if_set(&mut parts, "--flag", "  ");
        assert!(parts.is_empty());
    }

    #[test]
    fn test_push_if_set_value() {
        let mut parts = vec![];
        push_if_set(&mut parts, "--flag", "val");
        assert_eq!(parts, vec!["--flag val"]);
    }
}
