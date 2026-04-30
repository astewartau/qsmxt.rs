use std::path::PathBuf;

use crate::cli::*;
use super::app::App;

pub fn build_command_string(app: &App) -> String {
    let form = &app.form;
    let ps = &app.pipeline_state;
    let defaults = super::app::PipelineFormState::default();
    let is_slurm = form.execution_mode == 1;
    let mut parts = vec![
        "qsmxt".to_string(),
        if is_slurm { "slurm".to_string() } else { "run".to_string() },
    ];

    // Positional args
    if form.bids_dir.is_empty() {
        parts.push("<bids_dir>".to_string());
    } else {
        parts.push(form.bids_dir.clone());
    }
    if !form.output_dir.is_empty() {
        parts.push(form.output_dir.clone());
    }

    // Config file
    if !form.config_file.is_empty() {
        parts.push(format!("--config {}", form.config_file));
    }

    // Filters (from tree selection / include/exclude patterns)
    let (include, exclude) = app.filter_state.get_include_exclude();
    if let Some(ref inc) = include {
        parts.push(format!("--include {}", inc.join(" ")));
    }
    if let Some(ref exc) = exclude {
        parts.push(format!("--exclude {}", exc.join(" ")));
    }
    if !app.filter_state.num_echoes.is_empty() {
        parts.push(format!("--num-echoes {}", app.filter_state.num_echoes));
    }

    // Algorithm selects (only if changed from default)
    use super::app::{QSM_ALGO_OPTIONS, UNWRAP_OPTIONS, BF_OPTIONS, QSM_REF_OPTIONS};
    if ps.qsm_algorithm != defaults.qsm_algorithm {
        parts.push(format!("--qsm-algorithm {}", QSM_ALGO_OPTIONS[ps.qsm_algorithm]));
    }
    if ps.unwrapping_algorithm != defaults.unwrapping_algorithm {
        parts.push(format!("--unwrapping-algorithm {}", UNWRAP_OPTIONS[ps.unwrapping_algorithm]));
    }
    if ps.bf_algorithm != defaults.bf_algorithm {
        parts.push(format!("--bf-algorithm {}", BF_OPTIONS[ps.bf_algorithm]));
    }
    if ps.qsm_reference != defaults.qsm_reference {
        parts.push(format!("--qsm-reference {}", QSM_REF_OPTIONS[ps.qsm_reference]));
    }

    // Phase combination (only if changed from default)
    if ps.phase_combination != defaults.phase_combination {
        let val = if ps.phase_combination == 0 { "true" } else { "false" };
        parts.push(format!("--combine-phase {}", val));
    }

    // Parameters (only if changed from default)
    push_if_changed(&mut parts, "--obliquity-threshold", &ps.obliquity_threshold, &defaults.obliquity_threshold);

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

    // TSVD params
    push_if_changed(&mut parts, "--tsvd-threshold", &ps.tsvd_threshold, &defaults.tsvd_threshold);

    // iLSQR params
    push_if_changed(&mut parts, "--ilsqr-tol", &ps.ilsqr_tol, &defaults.ilsqr_tol);
    push_if_changed(&mut parts, "--ilsqr-max-iter", &ps.ilsqr_max_iter, &defaults.ilsqr_max_iter);

    // Tikhonov
    push_if_changed(&mut parts, "--tikhonov-lambda", &ps.tikhonov_lambda, &defaults.tikhonov_lambda);

    // NLTV
    push_if_changed(&mut parts, "--nltv-lambda", &ps.nltv_lambda, &defaults.nltv_lambda);
    push_if_changed(&mut parts, "--nltv-mu", &ps.nltv_mu, &defaults.nltv_mu);
    push_if_changed(&mut parts, "--nltv-tol", &ps.nltv_tol, &defaults.nltv_tol);
    push_if_changed(&mut parts, "--nltv-max-iter", &ps.nltv_max_iter, &defaults.nltv_max_iter);
    push_if_changed(&mut parts, "--nltv-newton-iter", &ps.nltv_newton_iter, &defaults.nltv_newton_iter);

    // MEDI
    push_if_changed(&mut parts, "--medi-lambda", &ps.medi_lambda, &defaults.medi_lambda);
    push_if_changed(&mut parts, "--medi-max-iter", &ps.medi_max_iter, &defaults.medi_max_iter);
    push_if_changed(&mut parts, "--medi-cg-max-iter", &ps.medi_cg_max_iter, &defaults.medi_cg_max_iter);
    push_if_changed(&mut parts, "--medi-cg-tol", &ps.medi_cg_tol, &defaults.medi_cg_tol);
    push_if_changed(&mut parts, "--medi-tol", &ps.medi_tol, &defaults.medi_tol);
    push_if_changed(&mut parts, "--medi-percentage", &ps.medi_percentage, &defaults.medi_percentage);
    push_if_changed(&mut parts, "--medi-smv-radius", &ps.medi_smv_radius, &defaults.medi_smv_radius);
    if ps.medi_smv != defaults.medi_smv
        && ps.medi_smv {
            parts.push("--medi-smv".to_string());
        }

    // BG removal params
    push_if_changed(&mut parts, "--vsharp-threshold", &ps.vsharp_threshold, &defaults.vsharp_threshold);
    push_if_changed(&mut parts, "--pdf-tol", &ps.pdf_tol, &defaults.pdf_tol);
    push_if_changed(&mut parts, "--lbv-tol", &ps.lbv_tol, &defaults.lbv_tol);
    push_if_changed(&mut parts, "--ismv-tol", &ps.ismv_tol, &defaults.ismv_tol);
    push_if_changed(&mut parts, "--ismv-max-iter", &ps.ismv_max_iter, &defaults.ismv_max_iter);
    push_if_changed(&mut parts, "--sharp-threshold", &ps.sharp_threshold, &defaults.sharp_threshold);

    // TGV params
    push_if_changed(&mut parts, "--tgv-iterations", &ps.tgv_iterations, &defaults.tgv_iterations);
    push_if_changed(&mut parts, "--tgv-erosions", &ps.tgv_erosions, &defaults.tgv_erosions);
    push_if_changed(&mut parts, "--tgv-alpha1", &ps.tgv_alpha1, &defaults.tgv_alpha1);
    push_if_changed(&mut parts, "--tgv-alpha0", &ps.tgv_alpha0, &defaults.tgv_alpha0);

    // QSMART params
    push_if_changed(&mut parts, "--qsmart-ilsqr-tol", &ps.qsmart_ilsqr_tol, &defaults.qsmart_ilsqr_tol);
    push_if_changed(&mut parts, "--qsmart-ilsqr-max-iter", &ps.qsmart_ilsqr_max_iter, &defaults.qsmart_ilsqr_max_iter);
    push_if_changed(&mut parts, "--qsmart-vasc-sphere-radius", &ps.qsmart_vasc_sphere_radius, &defaults.qsmart_vasc_sphere_radius);
    push_if_changed(&mut parts, "--qsmart-sdf-spatial-radius", &ps.qsmart_sdf_spatial_radius, &defaults.qsmart_sdf_spatial_radius);

    // Mask: emit --mask-preset for known presets, --mask for custom
    let default_sections = super::app::PipelineFormState::default().mask_sections;
    if ps.mask_sections != default_sections {
        // Check if it matches the BET preset
        let bet_preset = vec![crate::pipeline::config::MaskSection {
            input: crate::pipeline::config::MaskingInput::Magnitude,
            generator: crate::pipeline::config::MaskOp::Bet { fractional_intensity: 0.5 },
            refinements: vec![crate::pipeline::config::MaskOp::Erode { iterations: 2 }],
        }];
        if ps.mask_sections == bet_preset {
            parts.push("--mask-preset bet".to_string());
        } else {
            for section in ps.mask_sections.iter() {
                let mut section_parts = vec![format!("{}", section.input)];
                for op in &section.all_ops() {
                    section_parts.push(format!("{}", op));
                }
                parts.push(format!("--mask {}", section_parts.join(",")));
            }
        }
    }

    // Execution flags (only if non-default — defaults are all false/empty)
    if !ps.do_qsm {
        parts.push("--no-qsm".to_string());
    }
    if form.do_swi {
        parts.push("--do-swi".to_string());
        // SWI params (only if changed from default)
        let swi_scaling_options = ["tanh", "negative-tanh", "positive", "negative", "triangular"];
        let swi_scaling = swi_scaling_options.get(form.swi_scaling).unwrap_or(&"tanh");
        if *swi_scaling != "tanh" {
            parts.push(format!("--swi-scaling {}", swi_scaling));
        }
        let form_defaults = super::app::RunForm::default();
        push_if_changed(&mut parts, "--swi-strength", &form.swi_strength, &form_defaults.swi_strength);
        // HP sigma: emit if any component differs from default
        if form.swi_hp_sigma_x != form_defaults.swi_hp_sigma_x
            || form.swi_hp_sigma_y != form_defaults.swi_hp_sigma_y
            || form.swi_hp_sigma_z != form_defaults.swi_hp_sigma_z {
            parts.push(format!("--swi-hp-sigma {} {} {}",
                form.swi_hp_sigma_x.trim(), form.swi_hp_sigma_y.trim(), form.swi_hp_sigma_z.trim()));
        }
        push_if_changed(&mut parts, "--swi-mip-window", &form.swi_mip_window, &form_defaults.swi_mip_window);
    }
    if form.do_t2starmap {
        parts.push("--do-t2starmap".to_string());
    }
    if form.do_r2starmap {
        parts.push("--do-r2starmap".to_string());
    }
    if ps.inhomogeneity_correction != defaults.inhomogeneity_correction {
        if ps.inhomogeneity_correction {
            parts.push("--inhomogeneity-correction".to_string());
        } else {
            parts.push("--no-inhomogeneity-correction".to_string());
        }
    }
    if is_slurm {
        // SLURM-specific flags
        if !form.slurm_account.trim().is_empty() {
            parts.push(format!("--account {}", form.slurm_account.trim()));
        } else {
            parts.push("--account <account>".to_string());
        }
        let slurm_defaults = super::app::RunForm::default();
        if !form.slurm_partition.trim().is_empty() {
            parts.push(format!("--partition {}", form.slurm_partition.trim()));
        }
        push_if_changed(&mut parts, "--time", &form.slurm_time, &slurm_defaults.slurm_time);
        push_if_changed(&mut parts, "--mem", &form.slurm_mem, &slurm_defaults.slurm_mem);
        push_if_changed(&mut parts, "--cpus-per-task", &form.slurm_cpus, &slurm_defaults.slurm_cpus);
        if form.slurm_submit {
            parts.push("--submit".to_string());
        }
    } else {
        if form.dry_run {
            parts.push("--dry".to_string());
        }
        if form.debug {
            parts.push("--debug".to_string());
        }
        if !form.n_procs.trim().is_empty() {
            parts.push(format!("--n-procs {}", form.n_procs.trim()));
        }
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
    if form.bids_dir.is_empty() {
        return Err(crate::error::QsmxtError::Config(
            "BIDS directory is required".to_string(),
        ));
    }

    let qsm_options = [
        QsmAlgorithmArg::Rts,
        QsmAlgorithmArg::Tv,
        QsmAlgorithmArg::Tkd,
        QsmAlgorithmArg::Tsvd,
        QsmAlgorithmArg::Tgv,
        QsmAlgorithmArg::Tikhonov,
        QsmAlgorithmArg::Nltv,
        QsmAlgorithmArg::Medi,
        QsmAlgorithmArg::Ilsqr,
        QsmAlgorithmArg::Qsmart,
    ];
    let unwrap_options = [UnwrapAlgorithmArg::Romeo, UnwrapAlgorithmArg::Laplacian];
    let bf_options = [
        BfAlgorithmArg::Vsharp,
        BfAlgorithmArg::Pdf,
        BfAlgorithmArg::Lbv,
        BfAlgorithmArg::Ismv,
        BfAlgorithmArg::Sharp,
    ];
    Ok(RunArgs {
        bids_dir: PathBuf::from(&form.bids_dir),
        output_dir: if form.output_dir.is_empty() { None } else { Some(PathBuf::from(&form.output_dir)) },
        config: parse_optional_path(&form.config_file),
        include: app.filter_state.get_include_exclude().0,
        exclude: app.filter_state.get_include_exclude().1,
        num_echoes: parse_optional_usize(&app.filter_state.num_echoes),
        qsm_algorithm: Some(qsm_options[ps.qsm_algorithm]),
        unwrapping_algorithm: Some(unwrap_options[ps.unwrapping_algorithm]),
        bf_algorithm: Some(bf_options[ps.bf_algorithm]),
        masking_algorithm: None,
        masking_input: None,
        combine_phase: Some(ps.phase_combination == 0), // 0=mcpc3ds (true), 1=linear_fit (false)
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
        mask_erosions: None,
        rts_params: crate::cli::RtsParamArgs {
            rts_delta: parse_optional_f64(&ps.rts_delta),
            rts_mu: parse_optional_f64(&ps.rts_mu),
            rts_tol: parse_optional_f64(&ps.rts_tol),
            rts_rho: parse_optional_f64(&ps.rts_rho),
            rts_max_iter: parse_optional_usize(&ps.rts_max_iter),
            rts_lsmr_iter: parse_optional_usize(&ps.rts_lsmr_iter),
        },
        tv_params: crate::cli::TvParamArgs {
            tv_lambda: parse_optional_f64(&ps.tv_lambda),
            tv_rho: parse_optional_f64(&ps.tv_rho),
            tv_tol: parse_optional_f64(&ps.tv_tol),
            tv_max_iter: parse_optional_usize(&ps.tv_max_iter),
        },
        tkd_params: crate::cli::TkdParamArgs {
            tkd_threshold: parse_optional_f64(&ps.tkd_threshold),
        },
        tsvd_params: crate::cli::TsvdParamArgs {
            tsvd_threshold: parse_optional_f64(&ps.tsvd_threshold),
        },
        tgv_params: crate::cli::TgvParamArgs {
            tgv_iterations: parse_optional_usize(&ps.tgv_iterations),
            tgv_erosions: parse_optional_usize(&ps.tgv_erosions),
            tgv_alpha1: parse_optional_f64(&ps.tgv_alpha1),
            tgv_alpha0: parse_optional_f64(&ps.tgv_alpha0),
            tgv_step_size: None,
            tgv_tol: None,
        },
        tikhonov_params: crate::cli::TikhonovParamArgs {
            tikhonov_lambda: parse_optional_f64(&ps.tikhonov_lambda),
        },
        nltv_params: crate::cli::NltvParamArgs {
            nltv_lambda: parse_optional_f64(&ps.nltv_lambda),
            nltv_mu: parse_optional_f64(&ps.nltv_mu),
            nltv_tol: parse_optional_f64(&ps.nltv_tol),
            nltv_max_iter: parse_optional_usize(&ps.nltv_max_iter),
            nltv_newton_iter: parse_optional_usize(&ps.nltv_newton_iter),
        },
        medi_params: crate::cli::MediParamArgs {
            medi_lambda: parse_optional_f64(&ps.medi_lambda),
            medi_merit: None,
            medi_smv: ps.medi_smv,
            medi_smv_radius: parse_optional_f64(&ps.medi_smv_radius),
            medi_data_weighting: None,
            medi_percentage: parse_optional_f64(&ps.medi_percentage),
            medi_cg_tol: parse_optional_f64(&ps.medi_cg_tol),
            medi_cg_max_iter: parse_optional_usize(&ps.medi_cg_max_iter),
            medi_max_iter: parse_optional_usize(&ps.medi_max_iter),
            medi_tol: parse_optional_f64(&ps.medi_tol),
        },
        ilsqr_params: crate::cli::IlsqrParamArgs {
            ilsqr_tol: parse_optional_f64(&ps.ilsqr_tol),
            ilsqr_max_iter: parse_optional_usize(&ps.ilsqr_max_iter),
        },
        qsmart_params: crate::cli::QsmartParamArgs {
            qsmart_ilsqr_tol: parse_optional_f64(&ps.qsmart_ilsqr_tol),
            qsmart_ilsqr_max_iter: parse_optional_usize(&ps.qsmart_ilsqr_max_iter),
            qsmart_vasc_sphere_radius: ps.qsmart_vasc_sphere_radius.trim().parse::<i32>().ok(),
            qsmart_sdf_spatial_radius: ps.qsmart_sdf_spatial_radius.trim().parse::<i32>().ok(),
        },
        vsharp_params: crate::cli::VsharpParamArgs {
            vsharp_threshold: parse_optional_f64(&ps.vsharp_threshold),
            vsharp_max_radius_factor: None,
            vsharp_min_radius_factor: None,
        },
        pdf_params: crate::cli::PdfParamArgs {
            pdf_tol: parse_optional_f64(&ps.pdf_tol),
        },
        lbv_params: crate::cli::LbvParamArgs {
            lbv_tol: parse_optional_f64(&ps.lbv_tol),
        },
        ismv_params: crate::cli::IsmvParamArgs {
            ismv_tol: parse_optional_f64(&ps.ismv_tol),
            ismv_max_iter: parse_optional_usize(&ps.ismv_max_iter),
            ismv_radius_factor: None,
        },
        sharp_params: crate::cli::SharpParamArgs {
            sharp_threshold: parse_optional_f64(&ps.sharp_threshold),
            sharp_radius_factor: None,
        },
        romeo_params: crate::cli::RomeoParamArgs {
            no_romeo_phase_gradient_coherence: !ps.romeo_phase_gradient_coherence,
            no_romeo_mag_coherence: !ps.romeo_mag_coherence,
            no_romeo_mag_weight: !ps.romeo_mag_weight,
        },
        swi_params: crate::cli::SwiParamArgs {
            swi_hp_sigma: {
                let x: Option<f64> = form.swi_hp_sigma_x.trim().parse().ok();
                let y: Option<f64> = form.swi_hp_sigma_y.trim().parse().ok();
                let z: Option<f64> = form.swi_hp_sigma_z.trim().parse().ok();
                match (x, y, z) {
                    (Some(a), Some(b), Some(c)) => Some(vec![a, b, c]),
                    _ => None,
                }
            },
            swi_scaling: {
                let scaling_options = ["tanh", "negative-tanh", "positive", "negative", "triangular"];
                Some(scaling_options.get(form.swi_scaling).unwrap_or(&"tanh").to_string())
            },
            swi_strength: parse_optional_f64(&form.swi_strength),
            swi_mip_window: parse_optional_usize(&form.swi_mip_window),
        },
        mcpc3ds_sigma: {
            let vals: Vec<f64> = ps.mcpc3ds_sigma.split_whitespace()
                .filter_map(|w| w.parse().ok())
                .collect();
            if vals.is_empty() { None } else { Some(vals) }
        },
        n_procs: parse_optional_usize(&form.n_procs),
        homogeneity_sigma_mm: None,
        homogeneity_nbox: None,
        linear_fit_reliability_threshold: None,
        no_qsm: !ps.do_qsm,
        do_swi: form.do_swi,
        do_t2starmap: form.do_t2starmap,
        do_r2starmap: form.do_r2starmap,
        inhomogeneity_correction: ps.inhomogeneity_correction,
        no_inhomogeneity_correction: !ps.inhomogeneity_correction,
        obliquity_threshold: parse_optional_f64(&ps.obliquity_threshold),
        mask_preset: None,
        mask_sections_cli: {
            let secs: Vec<String> = ps.mask_sections.iter().map(|section| {
                let mut parts = vec![format!("{}", section.input)];
                for op in &section.all_ops() {
                    parts.push(format!("{}", op));
                }
                parts.join(",")
            }).collect();
            if secs.is_empty() { None } else { Some(secs) }
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

pub fn build_slurm_args(app: &App) -> crate::Result<SlurmArgs> {
    let form = &app.form;
    if form.bids_dir.is_empty() {
        return Err(crate::error::QsmxtError::Config(
            "BIDS directory is required".to_string(),
        ));
    }
    if form.slurm_account.trim().is_empty() {
        return Err(crate::error::QsmxtError::Config(
            "SLURM account is required".to_string(),
        ));
    }

    let defaults = super::app::RunForm::default();
    let (include, exclude) = app.filter_state.get_include_exclude();
    Ok(SlurmArgs {
        bids_dir: PathBuf::from(&form.bids_dir),
        output_dir: if form.output_dir.is_empty() { None } else { Some(PathBuf::from(&form.output_dir)) },
        account: form.slurm_account.trim().to_string(),
        partition: if form.slurm_partition.trim().is_empty() { None } else { Some(form.slurm_partition.trim().to_string()) },
        config: parse_optional_path(&form.config_file),
        time: if form.slurm_time.trim().is_empty() { defaults.slurm_time.clone() } else { form.slurm_time.trim().to_string() },
        mem: form.slurm_mem.trim().parse().unwrap_or(32),
        cpus_per_task: form.slurm_cpus.trim().parse().unwrap_or(4),
        submit: form.slurm_submit,
        include,
        exclude,
        num_echoes: parse_optional_usize(&app.filter_state.num_echoes),
    })
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
        assert!(!cmd.contains("<output_dir>"));
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
        // With no changes, only positional bids_dir should appear (output_dir is optional)
        assert!(cmd.starts_with("qsmxt run <bids_dir>"));
        assert!(!cmd.contains("--rts-delta"));
        assert!(!cmd.contains("--qsm-algorithm"));
        assert!(!cmd.contains("--n-procs"));
        assert!(!cmd.contains("--mask-op"));
    }

    #[test]
    fn test_command_string_phase_combination() {
        let mut app = default_app();
        app.pipeline_state.phase_combination = 1; // linear-fit (non-default)
        let cmd = build_command_string(&app);
        assert!(cmd.contains("--combine-phase false"));
    }

    #[test]
    fn test_command_string_execution_flags() {
        let mut app = default_app();
        app.form.do_swi = true;
        app.form.do_t2starmap = true;
        app.form.do_r2starmap = true;
        app.form.dry_run = true;
        app.form.debug = true;
        app.form.n_procs = "4".to_string();
        let cmd = build_command_string(&app);
        assert!(cmd.contains("--do-swi"));
        assert!(cmd.contains("--do-t2starmap"));
        assert!(cmd.contains("--do-r2starmap"));
        // inhomogeneity_correction is true by default, so shouldn't appear
        assert!(!cmd.contains("--inhomogeneity-correction"));
        assert!(cmd.contains("--dry"));
        assert!(cmd.contains("--debug"));
        assert!(cmd.contains("--n-procs 4"));
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
        assert_eq!(args.output_dir, Some(PathBuf::from("/out")));
        assert_eq!(args.qsm_algorithm, Some(crate::cli::QsmAlgorithmArg::Rts));
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
        assert_eq!(args.rts_params.rts_delta, Some(0.2));
        assert_eq!(args.tgv_params.tgv_iterations, Some(500));
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
        app.pipeline_state.inhomogeneity_correction = true;
        app.form.dry_run = true;
        app.form.debug = true;
        let args = build_run_args(&app).unwrap();
        assert!(args.do_swi);
        assert!(args.do_t2starmap);
        assert!(args.do_r2starmap);
        assert!(args.inhomogeneity_correction);
        assert!(args.dry);
        assert!(args.debug);
        assert_eq!(args.combine_phase, Some(true)); // default mcpc3ds
    }

    #[test]
    fn test_build_run_args_phase_combination_linear_fit() {
        let mut app = default_app();
        app.form.bids_dir = "/b".to_string();
        app.form.output_dir = "/o".to_string();
        app.pipeline_state.phase_combination = 1; // linear_fit
        let args = build_run_args(&app).unwrap();
        assert_eq!(args.combine_phase, Some(false));
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

    // --- SLURM command string ---

    #[test]
    fn test_command_string_slurm_mode() {
        let mut app = default_app();
        app.form.execution_mode = 1; // SLURM
        app.form.bids_dir = "/bids".to_string();
        app.form.slurm_account = "myacct".to_string();
        let cmd = build_command_string(&app);
        assert!(cmd.starts_with("qsmxt slurm"));
        assert!(cmd.contains("--account myacct"));
    }

    #[test]
    fn test_command_string_slurm_all_fields() {
        let mut app = default_app();
        app.form.execution_mode = 1;
        app.form.bids_dir = "/bids".to_string();
        app.form.slurm_account = "acct".to_string();
        app.form.slurm_partition = "gpu".to_string();
        app.form.slurm_time = "04:00:00".to_string();
        app.form.slurm_mem = "64".to_string();
        app.form.slurm_cpus = "8".to_string();
        app.form.slurm_submit = true;
        let cmd = build_command_string(&app);
        assert!(cmd.contains("--account acct"));
        assert!(cmd.contains("--partition gpu"));
        assert!(cmd.contains("--time 04:00:00"));
        assert!(cmd.contains("--mem 64"));
        assert!(cmd.contains("--cpus-per-task 8"));
        assert!(cmd.contains("--submit"));
    }

    #[test]
    fn test_command_string_slurm_defaults_omitted() {
        let mut app = default_app();
        app.form.execution_mode = 1;
        app.form.bids_dir = "/bids".to_string();
        app.form.slurm_account = "acct".to_string();
        // Default time/mem/cpus should not appear
        let cmd = build_command_string(&app);
        assert!(!cmd.contains("--time"));
        assert!(!cmd.contains("--mem"));
        assert!(!cmd.contains("--cpus-per-task"));
        assert!(!cmd.contains("--submit"));
        // Local-only flags should not appear
        assert!(!cmd.contains("--dry"));
        assert!(!cmd.contains("--n-procs"));
    }

    #[test]
    fn test_command_string_slurm_no_account_placeholder() {
        let mut app = default_app();
        app.form.execution_mode = 1;
        app.form.bids_dir = "/bids".to_string();
        let cmd = build_command_string(&app);
        assert!(cmd.contains("--account <account>"));
    }

    // --- build_slurm_args ---

    #[test]
    fn test_build_slurm_args_error_no_bids() {
        let app = default_app();
        assert!(build_slurm_args(&app).is_err());
    }

    #[test]
    fn test_build_slurm_args_error_no_account() {
        let mut app = default_app();
        app.form.bids_dir = "/bids".to_string();
        assert!(build_slurm_args(&app).is_err());
    }

    #[test]
    fn test_build_slurm_args_minimal() {
        let mut app = default_app();
        app.form.bids_dir = "/bids".to_string();
        app.form.slurm_account = "acct".to_string();
        let args = build_slurm_args(&app).unwrap();
        assert_eq!(args.bids_dir, PathBuf::from("/bids"));
        assert_eq!(args.account, "acct");
        assert_eq!(args.output_dir, None);
        assert_eq!(args.partition, None);
        assert_eq!(args.time, "02:00:00");
        assert_eq!(args.mem, 32);
        assert_eq!(args.cpus_per_task, 4);
        assert!(!args.submit);
    }

    #[test]
    fn test_build_slurm_args_full() {
        let mut app = default_app();
        app.form.bids_dir = "/bids".to_string();
        app.form.output_dir = "/out".to_string();
        app.form.slurm_account = "acct".to_string();
        app.form.slurm_partition = "gpu".to_string();
        app.form.slurm_time = "04:00:00".to_string();
        app.form.slurm_mem = "64".to_string();
        app.form.slurm_cpus = "8".to_string();
        app.form.slurm_submit = true;
        app.form.config_file = "config.toml".to_string();
        let args = build_slurm_args(&app).unwrap();
        assert_eq!(args.output_dir, Some(PathBuf::from("/out")));
        assert_eq!(args.partition, Some("gpu".to_string()));
        assert_eq!(args.time, "04:00:00");
        assert_eq!(args.mem, 64);
        assert_eq!(args.cpus_per_task, 8);
        assert!(args.submit);
        assert_eq!(args.config, Some(PathBuf::from("config.toml")));
    }

    // --- output_dir optional ---

    #[test]
    fn test_build_run_args_output_dir_empty() {
        let mut app = default_app();
        app.form.bids_dir = "/bids".to_string();
        let args = build_run_args(&app).unwrap();
        assert_eq!(args.output_dir, None);
    }

    #[test]
    fn test_command_string_output_dir_omitted_when_empty() {
        let mut app = default_app();
        app.form.bids_dir = "/bids".to_string();
        let cmd = build_command_string(&app);
        assert_eq!(cmd.matches("/bids").count(), 1); // only bids_dir, no output_dir
    }
}
