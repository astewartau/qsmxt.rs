use std::path::PathBuf;

use crate::cli::*;
use super::app::App;

pub fn build_command_string(app: &App) -> String {
    let form = &app.form;
    let ps = &app.pipeline_state;
    let defaults = super::app::PipelineFormState::default();
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

    // Filters (from tree selection)
    let (subjects, sessions, acquisitions, runs) = app.filter_state.selected_filters();
    if let Some(subs) = &subjects {
        parts.push(format!("--subjects {}", subs.join(" ")));
    }
    if let Some(sess) = &sessions {
        parts.push(format!("--sessions {}", sess.join(" ")));
    }
    if let Some(acqs) = &acquisitions {
        parts.push(format!("--acquisitions {}", acqs.join(" ")));
    }
    if let Some(rs) = &runs {
        parts.push(format!("--runs {}", rs.join(" ")));
    }
    if !app.filter_state.num_echoes.is_empty() {
        parts.push(format!("--num-echoes {}", app.filter_state.num_echoes));
    }

    // Algorithm selects (only if changed from default)
    use super::app::{QSM_ALGO_OPTIONS, UNWRAP_OPTIONS, BF_OPTIONS, MASK_ALGO_OPTIONS, MASK_INPUT_OPTIONS, QSM_REF_OPTIONS};
    if ps.qsm_algorithm != defaults.qsm_algorithm {
        parts.push(format!("--qsm-algorithm {}", QSM_ALGO_OPTIONS[ps.qsm_algorithm]));
    }
    if ps.unwrapping_algorithm != defaults.unwrapping_algorithm {
        parts.push(format!("--unwrapping-algorithm {}", UNWRAP_OPTIONS[ps.unwrapping_algorithm]));
    }
    if ps.bf_algorithm != defaults.bf_algorithm {
        parts.push(format!("--bf-algorithm {}", BF_OPTIONS[ps.bf_algorithm]));
    }
    if ps.masking_algorithm != defaults.masking_algorithm {
        parts.push(format!("--masking-algorithm {}", MASK_ALGO_OPTIONS[ps.masking_algorithm]));
    }
    if ps.masking_input != defaults.masking_input {
        parts.push(format!("--masking-input {}", MASK_INPUT_OPTIONS[ps.masking_input]));
    }
    if ps.qsm_reference != defaults.qsm_reference {
        parts.push(format!("--qsm-reference {}", QSM_REF_OPTIONS[ps.qsm_reference]));
    }

    // Boolean toggles (only if changed from default)
    if ps.combine_phase != defaults.combine_phase {
        parts.push(format!("--combine-phase {}", ps.combine_phase));
    }

    // Parameters (only if changed from default)
    push_if_changed(&mut parts, "--obliquity-threshold", &ps.obliquity_threshold, &defaults.obliquity_threshold);
    push_if_changed(&mut parts, "--mask-erosions", &ps.mask_erosions, &defaults.mask_erosions);

    // BET params
    push_if_changed(&mut parts, "--bet-fractional-intensity", &ps.bet_fractional_intensity, &defaults.bet_fractional_intensity);
    push_if_changed(&mut parts, "--bet-smoothness", &ps.bet_smoothness, &defaults.bet_smoothness);
    push_if_changed(&mut parts, "--bet-gradient-threshold", &ps.bet_gradient_threshold, &defaults.bet_gradient_threshold);
    push_if_changed(&mut parts, "--bet-iterations", &ps.bet_iterations, &defaults.bet_iterations);
    push_if_changed(&mut parts, "--bet-subdivisions", &ps.bet_subdivisions, &defaults.bet_subdivisions);

    // RTS params
    push_if_changed(&mut parts, "--rts-delta", &ps.rts_delta, &defaults.rts_delta);
    push_if_changed(&mut parts, "--rts-mu", &ps.rts_mu, &defaults.rts_mu);
    push_if_changed(&mut parts, "--rts-tol", &ps.rts_tol, &defaults.rts_tol);
    push_if_changed(&mut parts, "--rts-rho", &ps.rts_rho, &defaults.rts_rho);
    push_if_changed(&mut parts, "--rts-max-iter", &ps.rts_max_iter, &defaults.rts_max_iter);
    push_if_changed(&mut parts, "--rts-lsmr-iter", &ps.rts_lsmr_iter, &defaults.rts_lsmr_iter);

    // TV params
    push_if_changed(&mut parts, "--tv-lambda", &ps.tv_lambda, &defaults.tv_lambda);
    push_if_changed(&mut parts, "--tv-rho", &ps.tv_rho, &defaults.tv_rho);
    push_if_changed(&mut parts, "--tv-tol", &ps.tv_tol, &defaults.tv_tol);
    push_if_changed(&mut parts, "--tv-max-iter", &ps.tv_max_iter, &defaults.tv_max_iter);

    // TKD params
    push_if_changed(&mut parts, "--tkd-threshold", &ps.tkd_threshold, &defaults.tkd_threshold);

    // TGV params
    push_if_changed(&mut parts, "--tgv-iterations", &ps.tgv_iterations, &defaults.tgv_iterations);
    push_if_changed(&mut parts, "--tgv-erosions", &ps.tgv_erosions, &defaults.tgv_erosions);
    push_if_changed(&mut parts, "--tgv-alpha1", &ps.tgv_alpha1, &defaults.tgv_alpha1);
    push_if_changed(&mut parts, "--tgv-alpha0", &ps.tgv_alpha0, &defaults.tgv_alpha0);

    // Mask ops (only if changed from the default for the selected masking algorithm)
    let default_ops = ps.default_mask_ops_for_current_masking();
    if ps.mask_ops != default_ops {
        for op in &ps.mask_ops {
            parts.push(format!("--mask-op {}", op));
        }
    }

    // Execution flags (only if non-default — defaults are all false/empty)
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
    if !form.n_procs.trim().is_empty() {
        parts.push(format!("--n-procs {}", form.n_procs.trim()));
    }

    parts.join(" ")
}

fn push_if_changed(parts: &mut Vec<String>, flag: &str, current: &str, default: &str) {
    if current.trim() != default.trim() {
        parts.push(format!("{} {}", flag, current.trim()));
    }
}

pub fn build_run_args(app: &App) -> crate::Result<RunArgs> {
    let form = &app.form;
    let ps = &app.pipeline_state;
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
        subjects: app.filter_state.selected_filters().0,
        sessions: app.filter_state.selected_filters().1,
        acquisitions: app.filter_state.selected_filters().2,
        runs: app.filter_state.selected_filters().3,
        num_echoes: parse_optional_usize(&app.filter_state.num_echoes),
        qsm_algorithm: Some(qsm_options[ps.qsm_algorithm]),
        unwrapping_algorithm: Some(unwrap_options[ps.unwrapping_algorithm]),
        bf_algorithm: Some(bf_options[ps.bf_algorithm]),
        masking_algorithm: Some(mask_algo_options[ps.masking_algorithm]),
        masking_input: Some(mask_input_options[ps.masking_input]),
        combine_phase: if ps.combine_phase { Some(true) } else { None },
        bet_fractional_intensity: parse_optional_f64(&ps.bet_fractional_intensity),
        bet_smoothness: parse_optional_f64(&ps.bet_smoothness),
        bet_gradient_threshold: parse_optional_f64(&ps.bet_gradient_threshold),
        bet_iterations: parse_optional_usize(&ps.bet_iterations),
        bet_subdivisions: parse_optional_usize(&ps.bet_subdivisions),
        qsm_reference: match ps.qsm_reference {
            0 => Some(crate::cli::QsmReferenceArg::Mean),
            1 => Some(crate::cli::QsmReferenceArg::None),
            _ => None,
        },
        tgv_alpha1: parse_optional_f64(&ps.tgv_alpha1),
        tgv_alpha0: parse_optional_f64(&ps.tgv_alpha0),
        mask_erosions: parse_optional_usize_vec(&ps.mask_erosions),
        rts_delta: parse_optional_f64(&ps.rts_delta),
        rts_mu: parse_optional_f64(&ps.rts_mu),
        rts_tol: parse_optional_f64(&ps.rts_tol),
        rts_rho: parse_optional_f64(&ps.rts_rho),
        rts_max_iter: parse_optional_usize(&ps.rts_max_iter),
        rts_lsmr_iter: parse_optional_usize(&ps.rts_lsmr_iter),
        tgv_iterations: parse_optional_usize(&ps.tgv_iterations),
        tgv_erosions: parse_optional_usize(&ps.tgv_erosions),
        tv_lambda: parse_optional_f64(&ps.tv_lambda),
        tv_rho: parse_optional_f64(&ps.tv_rho),
        tv_tol: parse_optional_f64(&ps.tv_tol),
        tv_max_iter: parse_optional_usize(&ps.tv_max_iter),
        tkd_threshold: parse_optional_f64(&ps.tkd_threshold),
        n_procs: parse_optional_usize(&form.n_procs),
        do_swi: form.do_swi,
        do_t2starmap: form.do_t2starmap,
        do_r2starmap: form.do_r2starmap,
        inhomogeneity_correction: form.inhomogeneity_correction,
        obliquity_threshold: parse_optional_f64(&ps.obliquity_threshold),
        mask_ops: if ps.mask_ops.is_empty() { None } else {
            Some(ps.mask_ops.iter().map(|op| format!("{}", op)).collect())
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

#[allow(dead_code)]
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
    use super::super::app::App;

    fn default_app() -> App {
        App::new()
    }

    // --- build_command_string ---

    #[test]
    fn test_command_string_minimal() {
        let app = default_app();
        let cmd = build_command_string(&app);
        assert!(cmd.starts_with("qsmxt run"));
        assert!(cmd.contains("<bids_dir>"));
        assert!(cmd.contains("<output_dir>"));
    }

    #[test]
    fn test_command_string_with_dirs() {
        let mut app = default_app();
        app.form.bids_dir = "/data/bids".to_string();
        app.form.output_dir = "/data/out".to_string();
        let cmd = build_command_string(&app);
        assert!(cmd.contains("/data/bids"));
        assert!(cmd.contains("/data/out"));
        assert!(!cmd.contains("<bids_dir>"));
    }

    #[test]
    fn test_command_string_with_preset() {
        let mut app = default_app();
        app.form.bids_dir = "/b".to_string();
        app.form.output_dir = "/o".to_string();
        app.form.preset = 1; // gre
        let cmd = build_command_string(&app);
        assert!(cmd.contains("--preset gre"));
    }

    #[test]
    fn test_command_string_no_preset_when_zero() {
        let app = default_app();
        let cmd = build_command_string(&app);
        assert!(!cmd.contains("--preset"));
    }

    #[test]
    fn test_command_string_with_config() {
        let mut app = default_app();
        app.form.config_file = "my.toml".to_string();
        let cmd = build_command_string(&app);
        assert!(cmd.contains("--config my.toml"));
    }

    #[test]
    fn test_command_string_num_echoes() {
        let mut app = default_app();
        app.filter_state.num_echoes = "4".to_string();
        let cmd = build_command_string(&app);
        assert!(cmd.contains("--num-echoes 4"));
    }

    #[test]
    fn test_command_string_algorithms() {
        let mut app = default_app();
        app.pipeline_state.qsm_algorithm = 2; // tkd
        app.pipeline_state.unwrapping_algorithm = 1; // laplacian
        app.pipeline_state.bf_algorithm = 3; // ismv
        app.pipeline_state.masking_algorithm = 0; // bet
        app.pipeline_state.masking_input = 3; // phase-quality
        let cmd = build_command_string(&app);
        assert!(cmd.contains("--qsm-algorithm tkd"));
        assert!(cmd.contains("--unwrapping-algorithm laplacian"));
        assert!(cmd.contains("--bf-algorithm ismv"));
        assert!(cmd.contains("--masking-algorithm bet"));
        assert!(cmd.contains("--masking-input phase-quality"));
    }

    #[test]
    fn test_command_string_parameters() {
        let mut app = default_app();
        app.pipeline_state.bet_fractional_intensity = "0.3".to_string();
        app.pipeline_state.rts_delta = "0.2".to_string();
        app.pipeline_state.obliquity_threshold = "5".to_string();
        let cmd = build_command_string(&app);
        assert!(cmd.contains("--bet-fractional-intensity 0.3"));
        assert!(cmd.contains("--rts-delta 0.2"));
        assert!(cmd.contains("--obliquity-threshold 5"));
    }

    #[test]
    fn test_command_string_no_defaults_shown() {
        let app = default_app();
        let cmd = build_command_string(&app);
        // With no changes, only positional args should appear
        assert!(cmd.starts_with("qsmxt run <bids_dir> <output_dir>"));
        assert!(!cmd.contains("--rts-delta"));
        assert!(!cmd.contains("--qsm-algorithm"));
        assert!(!cmd.contains("--n-procs"));
        assert!(!cmd.contains("--mask-op"));
    }

    #[test]
    fn test_command_string_combine_phase() {
        let mut app = default_app();
        app.pipeline_state.combine_phase = true;
        let cmd = build_command_string(&app);
        assert!(cmd.contains("--combine-phase true"));
    }

    #[test]
    fn test_command_string_execution_flags() {
        let mut app = default_app();
        app.form.do_swi = true;
        app.form.do_t2starmap = true;
        app.form.do_r2starmap = true;
        app.form.inhomogeneity_correction = true;
        app.form.dry_run = true;
        app.form.debug = true;
        app.form.n_procs = "4".to_string();
        let cmd = build_command_string(&app);
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
        let mut app = default_app();
        app.pipeline_state.mask_ops = vec![
            crate::pipeline::config::MaskOp::Erode { iterations: 2 },
            crate::pipeline::config::MaskOp::Dilate { iterations: 1 },
        ];
        let cmd = build_command_string(&app);
        assert!(cmd.contains("--mask-op erode:2"));
        assert!(cmd.contains("--mask-op dilate:1"));
    }

    // --- build_run_args ---

    #[test]
    fn test_build_run_args_error_when_empty() {
        let app = default_app();
        let result = build_run_args(&app);
        assert!(result.is_err());
    }

    #[test]
    fn test_build_run_args_minimal() {
        let mut app = default_app();
        app.form.bids_dir = "/bids".to_string();
        app.form.output_dir = "/out".to_string();
        let args = build_run_args(&app).unwrap();
        assert_eq!(args.bids_dir, PathBuf::from("/bids"));
        assert_eq!(args.output_dir, PathBuf::from("/out"));
        assert_eq!(args.preset, None);
        assert_eq!(args.qsm_algorithm, Some(crate::cli::QsmAlgorithmArg::Rts));
    }

    #[test]
    fn test_build_run_args_with_preset() {
        let mut app = default_app();
        app.form.bids_dir = "/b".to_string();
        app.form.output_dir = "/o".to_string();
        app.form.preset = 2; // epi
        let args = build_run_args(&app).unwrap();
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
            let mut app = default_app();
            app.form.bids_dir = "/b".to_string();
            app.form.output_dir = "/o".to_string();
            app.form.preset = idx;
            let args = build_run_args(&app).unwrap();
            assert_eq!(args.preset, expected);
        }
    }

    #[test]
    fn test_build_run_args_algorithms() {
        let mut app = default_app();
        app.form.bids_dir = "/b".to_string();
        app.form.output_dir = "/o".to_string();
        app.pipeline_state.qsm_algorithm = 3; // tgv
        app.pipeline_state.unwrapping_algorithm = 1; // laplacian
        app.pipeline_state.bf_algorithm = 2; // lbv
        app.pipeline_state.masking_algorithm = 0; // bet
        app.pipeline_state.masking_input = 2; // magnitude-last
        let args = build_run_args(&app).unwrap();
        assert_eq!(args.qsm_algorithm, Some(crate::cli::QsmAlgorithmArg::Tgv));
        assert_eq!(args.unwrapping_algorithm, Some(crate::cli::UnwrapAlgorithmArg::Laplacian));
        assert_eq!(args.bf_algorithm, Some(crate::cli::BfAlgorithmArg::Lbv));
        assert_eq!(args.masking_algorithm, Some(crate::cli::MaskAlgorithmArg::Bet));
        assert_eq!(args.masking_input, Some(crate::cli::MaskInputArg::MagnitudeLast));
    }

    #[test]
    fn test_build_run_args_num_echoes() {
        let mut app = default_app();
        app.form.bids_dir = "/b".to_string();
        app.form.output_dir = "/o".to_string();
        app.filter_state.num_echoes = "4".to_string();
        let args = build_run_args(&app).unwrap();
        assert_eq!(args.num_echoes, Some(4));
    }

    #[test]
    fn test_build_run_args_numeric_params() {
        let mut app = default_app();
        app.form.bids_dir = "/b".to_string();
        app.form.output_dir = "/o".to_string();
        app.pipeline_state.bet_fractional_intensity = "0.3".to_string();
        app.pipeline_state.rts_delta = "0.2".to_string();
        app.pipeline_state.rts_mu = "1e5".to_string();
        app.pipeline_state.rts_tol = "1e-4".to_string();
        app.pipeline_state.tgv_iterations = "500".to_string();
        app.pipeline_state.tgv_erosions = "2".to_string();
        app.pipeline_state.tv_lambda = "0.001".to_string();
        app.pipeline_state.tkd_threshold = "0.15".to_string();
        app.form.n_procs = "8".to_string();
        let args = build_run_args(&app).unwrap();
        assert_eq!(args.bet_fractional_intensity, Some(0.3));
        assert_eq!(args.rts_delta, Some(0.2));
        assert_eq!(args.tgv_iterations, Some(500));
        assert_eq!(args.n_procs, Some(8));
    }

    #[test]
    fn test_build_run_args_flags() {
        let mut app = default_app();
        app.form.bids_dir = "/b".to_string();
        app.form.output_dir = "/o".to_string();
        app.form.do_swi = true;
        app.form.do_t2starmap = true;
        app.form.do_r2starmap = true;
        app.form.inhomogeneity_correction = true;
        app.form.dry_run = true;
        app.form.debug = true;
        app.pipeline_state.combine_phase = true;
        let args = build_run_args(&app).unwrap();
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
        let mut app = default_app();
        app.form.bids_dir = "/b".to_string();
        app.form.output_dir = "/o".to_string();
        app.pipeline_state.combine_phase = false;
        let args = build_run_args(&app).unwrap();
        assert_eq!(args.combine_phase, None);
    }

    #[test]
    fn test_build_run_args_mask_ops() {
        let mut app = default_app();
        app.form.bids_dir = "/b".to_string();
        app.form.output_dir = "/o".to_string();
        app.pipeline_state.mask_ops = vec![
            crate::pipeline::config::MaskOp::Erode { iterations: 2 },
        ];
        let args = build_run_args(&app).unwrap();
        assert!(args.mask_ops.is_some());
    }

    #[test]
    fn test_build_run_args_default_mask_ops() {
        let mut app = default_app();
        app.form.bids_dir = "/b".to_string();
        app.form.output_dir = "/o".to_string();
        let args = build_run_args(&app).unwrap();
        // Default pipeline_state has threshold mask_ops
        assert!(args.mask_ops.is_some());
    }

    #[test]
    fn test_build_run_args_config_file() {
        let mut app = default_app();
        app.form.bids_dir = "/b".to_string();
        app.form.output_dir = "/o".to_string();
        app.form.config_file = "pipeline.toml".to_string();
        let args = build_run_args(&app).unwrap();
        assert_eq!(args.config, Some(PathBuf::from("pipeline.toml")));
    }

    #[test]
    fn test_build_run_args_mask_erosions() {
        let mut app = default_app();
        app.form.bids_dir = "/b".to_string();
        app.form.output_dir = "/o".to_string();
        app.pipeline_state.mask_erosions = "2 3 4".to_string();
        let args = build_run_args(&app).unwrap();
        assert_eq!(args.mask_erosions, Some(vec![2, 3, 4]));
    }

    #[test]
    fn test_build_run_args_obliquity() {
        let mut app = default_app();
        app.form.bids_dir = "/b".to_string();
        app.form.output_dir = "/o".to_string();
        app.pipeline_state.obliquity_threshold = "5.0".to_string();
        let args = build_run_args(&app).unwrap();
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
        assert_eq!(parse_optional_usize_vec("1 abc 3"), Some(vec![1, 3]));
    }

    #[test]
    fn test_push_if_changed_same() {
        let mut parts = vec![];
        push_if_changed(&mut parts, "--flag", "val", "val");
        assert!(parts.is_empty());
    }

    #[test]
    fn test_push_if_changed_different() {
        let mut parts = vec![];
        push_if_changed(&mut parts, "--flag", "new", "old");
        assert_eq!(parts, vec!["--flag new"]);
    }
}
