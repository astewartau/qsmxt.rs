pub mod common;
pub mod bet;
pub mod bgremove;
pub mod close;
pub mod dilate;
pub mod fill_holes;
pub mod homogeneity;
pub mod init;
pub mod invert;
pub mod mask;
pub mod presets;
pub mod quality_map;
pub mod r2star;
pub mod resample;
pub mod run;
pub mod slurm;
pub mod smooth_mask;
pub mod swi;
pub mod t2star;
pub mod unwrap;
pub mod validate;

#[cfg(test)]
mod integration_tests {
    use crate::cli::*;
    use crate::testutils;
    use std::path::PathBuf;

    // --- Mask ---

    #[test]
    fn test_mask_otsu() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("mag.nii");
        let output = dir.path().join("mask.nii");
        testutils::write_magnitude(&input);

        super::mask::execute(MaskArgs {
            input,
            output: output.clone(),
            method: ThresholdMethod::Otsu,
            threshold: None,
            erosions: 0,
        }).unwrap();
        assert!(output.exists());
    }

    #[test]
    fn test_mask_value_threshold() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("mag.nii");
        let output = dir.path().join("mask.nii");
        testutils::write_magnitude(&input);

        super::mask::execute(MaskArgs {
            input,
            output: output.clone(),
            method: ThresholdMethod::Value,
            threshold: Some(500.0),
            erosions: 1,
        }).unwrap();
        assert!(output.exists());
    }

    #[test]
    fn test_mask_value_requires_threshold() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("mag.nii");
        let output = dir.path().join("mask.nii");
        testutils::write_magnitude(&input);

        let result = super::mask::execute(MaskArgs {
            input,
            output,
            method: ThresholdMethod::Value,
            threshold: None,
            erosions: 0,
        });
        assert!(result.is_err());
    }

    // --- BET ---

    #[test]
    fn test_bet() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("mag.nii");
        let output = dir.path().join("mask.nii");
        testutils::write_magnitude(&input);

        super::bet::execute(BetArgs {
            input,
            output: output.clone(),
            fractional_intensity: 0.5,
            smoothness: 1.0,
            gradient_threshold: 0.0,
            iterations: 100, // fewer for speed
            subdivisions: 2, // fewer for speed
        }).unwrap();
        assert!(output.exists());
    }

    // --- Dilate ---

    #[test]
    fn test_dilate() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("mask.nii");
        let output = dir.path().join("dilated.nii");
        testutils::write_mask(&input);

        super::dilate::execute(DilateArgs {
            input,
            output: output.clone(),
            iterations: 1,
        }).unwrap();
        assert!(output.exists());
    }

    // --- Close ---

    #[test]
    fn test_close() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("mask.nii");
        let output = dir.path().join("closed.nii");
        testutils::write_mask(&input);

        super::close::execute(CloseArgs {
            input,
            output: output.clone(),
            radius: 1,
        }).unwrap();
        assert!(output.exists());
    }

    // --- Fill Holes ---

    #[test]
    fn test_fill_holes() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("mask.nii");
        let output = dir.path().join("filled.nii");
        testutils::write_mask(&input);

        super::fill_holes::execute(FillHolesArgs {
            input,
            output: output.clone(),
            max_size: 1000,
        }).unwrap();
        assert!(output.exists());
    }

    // --- Smooth Mask ---

    #[test]
    fn test_smooth_mask() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("mask.nii");
        let output = dir.path().join("smoothed.nii");
        testutils::write_mask(&input);

        super::smooth_mask::execute(SmoothMaskArgs {
            input,
            output: output.clone(),
            sigma: 2.0,
        }).unwrap();
        assert!(output.exists());
    }

    // --- Unwrap ---

    #[test]
    fn test_unwrap_laplacian() {
        let dir = tempfile::tempdir().unwrap();
        let phase = dir.path().join("phase.nii");
        let mask = dir.path().join("mask.nii");
        let output = dir.path().join("unwrapped.nii");
        testutils::write_phase(&phase);
        testutils::write_mask(&mask);

        super::unwrap::execute(UnwrapArgs {
            input: phase,
            mask,
            output: output.clone(),
            algorithm: UnwrapAlgorithmArg::Laplacian,
            magnitude: None,
        }).unwrap();
        assert!(output.exists());
    }

    #[test]
    fn test_unwrap_romeo() {
        let dir = tempfile::tempdir().unwrap();
        let phase = dir.path().join("phase.nii");
        let mask = dir.path().join("mask.nii");
        let mag = dir.path().join("mag.nii");
        let output = dir.path().join("unwrapped.nii");
        testutils::write_phase(&phase);
        testutils::write_mask(&mask);
        testutils::write_magnitude(&mag);

        super::unwrap::execute(UnwrapArgs {
            input: phase,
            mask,
            output: output.clone(),
            algorithm: UnwrapAlgorithmArg::Romeo,
            magnitude: Some(mag),
        }).unwrap();
        assert!(output.exists());
    }

    // --- Background Removal ---

    #[test]
    fn test_bgremove_vsharp() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("field.nii");
        let mask = dir.path().join("mask.nii");
        let output = dir.path().join("local.nii");
        let output_mask = dir.path().join("bgmask.nii");
        testutils::write_field(&input);
        testutils::write_mask(&mask);

        super::bgremove::execute(BgremoveArgs {
            input,
            mask,
            output: output.clone(),
            algorithm: BfAlgorithmArg::Vsharp,
            b0_direction: vec![0.0, 0.0, 1.0],
            output_mask: Some(output_mask.clone()),
        }).unwrap();
        assert!(output.exists());
        assert!(output_mask.exists());
    }

    #[test]
    fn test_bgremove_pdf() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("field.nii");
        let mask = dir.path().join("mask.nii");
        let output = dir.path().join("local.nii");
        testutils::write_field(&input);
        testutils::write_mask(&mask);

        super::bgremove::execute(BgremoveArgs {
            input,
            mask,
            output: output.clone(),
            algorithm: BfAlgorithmArg::Pdf,
            b0_direction: vec![0.0, 0.0, 1.0],
            output_mask: None,
        }).unwrap();
        assert!(output.exists());
    }

    // --- Dipole Inversion ---

    #[test]
    fn test_invert_tkd() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("field.nii");
        let mask = dir.path().join("mask.nii");
        let output = dir.path().join("chi.nii");
        testutils::write_field(&input);
        testutils::write_mask(&mask);

        super::invert::execute(InvertArgs {
            input,
            mask,
            output: output.clone(),
            algorithm: QsmAlgorithmArg::Tkd,
            b0_direction: vec![0.0, 0.0, 1.0],
            rts_delta: 0.15,
            rts_mu: 1e5,
            rts_tol: 1e-4,
            rts_rho: 10.0,
            rts_max_iter: 20,
            rts_lsmr_iter: 4,
            tv_lambda: 1e-3,
            tv_rho: 0.02,
            tv_tol: 1e-3,
            tv_max_iter: 250,
            tkd_threshold: 0.15,
            tgv_iterations: 10,
            tgv_erosions: 1,
            field_strength: None,
            echo_time: None,
        }).unwrap();
        assert!(output.exists());
    }

    #[test]
    fn test_invert_tgv_requires_field_strength() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("field.nii");
        let mask = dir.path().join("mask.nii");
        let output = dir.path().join("chi.nii");
        testutils::write_field(&input);
        testutils::write_mask(&mask);

        let result = super::invert::execute(InvertArgs {
            input,
            mask,
            output,
            algorithm: QsmAlgorithmArg::Tgv,
            b0_direction: vec![0.0, 0.0, 1.0],
            rts_delta: 0.15,
            rts_mu: 1e5,
            rts_tol: 1e-4,
            rts_rho: 10.0,
            rts_max_iter: 20,
            rts_lsmr_iter: 4,
            tv_lambda: 1e-3,
            tv_rho: 0.02,
            tv_tol: 1e-3,
            tv_max_iter: 250,
            tkd_threshold: 0.15,
            tgv_iterations: 10,
            tgv_erosions: 1,
            field_strength: None,
            echo_time: Some(0.02),
        });
        assert!(result.is_err());
    }

    // --- Homogeneity ---

    #[test]
    fn test_homogeneity() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("mag.nii");
        let output = dir.path().join("corrected.nii");
        testutils::write_magnitude(&input);

        super::homogeneity::execute(HomogeneityArgs {
            input,
            output: output.clone(),
            sigma: 4.0,
            nbox: 2,
        }).unwrap();
        assert!(output.exists());
    }

    // --- Resample ---

    #[test]
    fn test_resample() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("vol.nii");
        let output = dir.path().join("resampled.nii");
        testutils::write_magnitude(&input);

        super::resample::execute(ResampleArgs {
            input,
            output: output.clone(),
        }).unwrap();
        assert!(output.exists());
    }

    // --- Quality Map ---

    #[test]
    fn test_quality_map() {
        let dir = tempfile::tempdir().unwrap();
        let phase = dir.path().join("phase.nii");
        let output = dir.path().join("quality.nii");
        testutils::write_phase(&phase);

        super::quality_map::execute(QualityMapArgs {
            phase,
            output: output.clone(),
            magnitude: None,
            phase2: None,
            te1: 0.02,
            te2: 0.04,
        }).unwrap();
        assert!(output.exists());
    }

    // --- SWI ---

    #[test]
    fn test_swi() {
        let dir = tempfile::tempdir().unwrap();
        let phase = dir.path().join("phase.nii");
        let mag = dir.path().join("mag.nii");
        let mask = dir.path().join("mask.nii");
        let output = dir.path().join("swi.nii");
        testutils::write_phase(&phase);
        testutils::write_magnitude(&mag);
        testutils::write_mask(&mask);

        super::swi::execute(SwiArgs {
            phase,
            magnitude: mag,
            mask,
            output: output.clone(),
            mip: false,
            mip_output: None,
        }).unwrap();
        assert!(output.exists());
    }

    // --- R2* ---

    #[test]
    fn test_r2star() {
        let dir = tempfile::tempdir().unwrap();
        let mask = dir.path().join("mask.nii");
        let output = dir.path().join("r2star.nii");
        testutils::write_mask(&mask);

        let mut inputs = Vec::new();
        for i in 1..=3 {
            let p = dir.path().join(format!("echo{}.nii", i));
            testutils::write_magnitude(&p);
            inputs.push(p);
        }

        super::r2star::execute(R2starArgs {
            inputs,
            mask,
            output: output.clone(),
            echo_times: vec![0.004, 0.008, 0.012],
        }).unwrap();
        assert!(output.exists());
    }

    #[test]
    fn test_r2star_mismatched_inputs() {
        let result = super::r2star::execute(R2starArgs {
            inputs: vec![PathBuf::from("a.nii"), PathBuf::from("b.nii"), PathBuf::from("c.nii")],
            mask: PathBuf::from("mask.nii"),
            output: PathBuf::from("out.nii"),
            echo_times: vec![0.004, 0.008], // only 2 times for 3 inputs
        });
        assert!(result.is_err());
    }

    // --- T2* ---

    #[test]
    fn test_t2star() {
        let dir = tempfile::tempdir().unwrap();
        let mask = dir.path().join("mask.nii");
        let output = dir.path().join("t2star.nii");
        testutils::write_mask(&mask);

        let mut inputs = Vec::new();
        for i in 1..=3 {
            let p = dir.path().join(format!("echo{}.nii", i));
            testutils::write_magnitude(&p);
            inputs.push(p);
        }

        super::t2star::execute(T2starArgs {
            inputs,
            mask,
            output: output.clone(),
            echo_times: vec![0.004, 0.008, 0.012],
        }).unwrap();
        assert!(output.exists());
    }

    // --- Init ---

    #[test]
    fn test_init_to_file() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("config.toml");
        super::init::execute(InitArgs {
            preset: Preset::Gre,
            output: Some(output.clone()),
        }).unwrap();
        assert!(output.exists());
        let content = std::fs::read_to_string(&output).unwrap();
        assert!(content.contains("qsm_algorithm"));
    }

    #[test]
    fn test_init_to_stdout() {
        super::init::execute(InitArgs {
            preset: Preset::Body,
            output: None,
        }).unwrap();
    }

    // --- Presets ---

    #[test]
    fn test_presets_list() {
        super::presets::execute(PresetsArgs { name: None }).unwrap();
    }

    #[test]
    fn test_presets_show_specific() {
        super::presets::execute(PresetsArgs { name: Some("gre".to_string()) }).unwrap();
    }

    #[test]
    fn test_presets_unknown() {
        super::presets::execute(PresetsArgs { name: Some("nonexistent".to_string()) }).unwrap();
    }

    // --- Validate ---

    #[test]
    fn test_validate_single_echo() {
        let dir = tempfile::tempdir().unwrap();
        testutils::create_single_echo_bids(dir.path());

        super::validate::execute(ValidateArgs {
            bids_dir: dir.path().to_path_buf(),
            subjects: None,
            sessions: None,
        }).unwrap();
    }

    #[test]
    fn test_validate_multi_echo() {
        let dir = tempfile::tempdir().unwrap();
        testutils::create_multi_echo_bids(dir.path());

        super::validate::execute(ValidateArgs {
            bids_dir: dir.path().to_path_buf(),
            subjects: None,
            sessions: None,
        }).unwrap();
    }

    #[test]
    fn test_validate_empty_dir() {
        let dir = tempfile::tempdir().unwrap();

        super::validate::execute(ValidateArgs {
            bids_dir: dir.path().to_path_buf(),
            subjects: None,
            sessions: None,
        }).unwrap(); // should not error, just print "no runs"
    }

    // --- Run (dry) ---

    #[test]
    fn test_run_dry_single_echo() {
        let dir = tempfile::tempdir().unwrap();
        let bids = dir.path().join("bids");
        let out = dir.path().join("out");
        testutils::create_single_echo_bids(&bids);

        super::run::execute(RunArgs {
            bids_dir: bids,
            output_dir: out,
            preset: Some(Preset::Gre),
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
            n_procs: Some(1),
            do_swi: false,
            do_t2starmap: false,
            do_r2starmap: false,
            inhomogeneity_correction: false,
            obliquity_threshold: None,
            mask_ops: None,
            dry: true,
            debug: false,
            mem_limit_gb: Some(4.0),
            no_mem_limit: false,
            force: false,
            clean_intermediates: false,
        }).unwrap();
    }

    #[test]
    fn test_run_dry_multi_echo() {
        let dir = tempfile::tempdir().unwrap();
        let bids = dir.path().join("bids");
        let out = dir.path().join("out");
        testutils::create_multi_echo_bids(&bids);

        super::run::execute(RunArgs {
            bids_dir: bids,
            output_dir: out,
            preset: None,
            config: None,
            subjects: None,
            sessions: None,
            acquisitions: None,
            runs: None,
            num_echoes: None,
            qsm_algorithm: Some(QsmAlgorithmArg::Tkd),
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
            n_procs: Some(1),
            do_swi: false,
            do_t2starmap: false,
            do_r2starmap: false,
            inhomogeneity_correction: false,
            obliquity_threshold: None,
            mask_ops: None,
            dry: true,
            debug: false,
            mem_limit_gb: None,
            no_mem_limit: true,
            force: false,
            clean_intermediates: false,
        }).unwrap();
    }

    #[test]
    fn test_run_dry_empty_bids() {
        let dir = tempfile::tempdir().unwrap();

        // Should not error, just log "no runs"
        super::run::execute(RunArgs {
            bids_dir: dir.path().to_path_buf(),
            output_dir: dir.path().join("out"),
            preset: Some(Preset::Gre),
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
            n_procs: Some(1),
            do_swi: false,
            do_t2starmap: false,
            do_r2starmap: false,
            inhomogeneity_correction: false,
            obliquity_threshold: None,
            mask_ops: None,
            dry: true,
            debug: false,
            mem_limit_gb: None,
            no_mem_limit: false,
            force: false,
            clean_intermediates: false,
        }).unwrap();
    }

    // --- Run (actual execution) ---

    #[test]
    fn test_run_single_echo_tkd() {
        let dir = tempfile::tempdir().unwrap();
        let bids = dir.path().join("bids");
        let out = dir.path().join("out");
        testutils::create_single_echo_bids(&bids);

        super::run::execute(RunArgs {
            bids_dir: bids,
            output_dir: out.clone(),
            preset: Some(Preset::Gre),
            config: None,
            subjects: None,
            sessions: None,
            acquisitions: None,
            runs: None,
            num_echoes: None,
            qsm_algorithm: Some(QsmAlgorithmArg::Tkd),
            unwrapping_algorithm: Some(UnwrapAlgorithmArg::Laplacian),
            bf_algorithm: Some(BfAlgorithmArg::Vsharp),
            masking_algorithm: Some(MaskAlgorithmArg::Threshold),
            masking_input: Some(MaskInputArg::MagnitudeFirst),
            combine_phase: None,
            bet_fractional_intensity: None,
            bet_smoothness: None,
            bet_gradient_threshold: None,
            bet_iterations: None,
            bet_subdivisions: None,
            qsm_reference: None,
            tgv_alpha1: None,
            tgv_alpha0: None,
            mask_erosions: Some(vec![1]),
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
            n_procs: Some(1),
            do_swi: false,
            do_t2starmap: false,
            do_r2starmap: false,
            inhomogeneity_correction: false,
            obliquity_threshold: None,
            mask_ops: None,
            dry: false,
            debug: false,
            mem_limit_gb: None,
            no_mem_limit: true,
            force: false,
            clean_intermediates: false,
        }).unwrap();

        // Check that output QSM file was created
        assert!(out.join("sub-1/anat/sub-1_Chimap.nii").exists());
        assert!(out.join("sub-1/anat/sub-1_mask.nii").exists());
        assert!(out.join("dataset_description.json").exists());
        assert!(out.join("pipeline_config.toml").exists());
    }

    #[test]
    fn test_run_multi_echo_with_extras() {
        let dir = tempfile::tempdir().unwrap();
        let bids = dir.path().join("bids");
        let out = dir.path().join("out");
        testutils::create_multi_echo_bids(&bids);

        super::run::execute(RunArgs {
            bids_dir: bids,
            output_dir: out.clone(),
            preset: Some(Preset::Gre),
            config: None,
            subjects: None,
            sessions: None,
            acquisitions: None,
            runs: None,
            num_echoes: None,
            qsm_algorithm: Some(QsmAlgorithmArg::Tkd),
            unwrapping_algorithm: Some(UnwrapAlgorithmArg::Laplacian),
            bf_algorithm: Some(BfAlgorithmArg::Vsharp),
            masking_algorithm: Some(MaskAlgorithmArg::Threshold),
            masking_input: Some(MaskInputArg::Magnitude),
            combine_phase: Some(true),
            bet_fractional_intensity: None,
            bet_smoothness: None,
            bet_gradient_threshold: None,
            bet_iterations: None,
            bet_subdivisions: None,
            qsm_reference: None,
            tgv_alpha1: None,
            tgv_alpha0: None,
            mask_erosions: Some(vec![1]),
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
            n_procs: Some(1),
            do_swi: true,
            do_t2starmap: true,
            do_r2starmap: true,
            inhomogeneity_correction: false,
            obliquity_threshold: None,
            mask_ops: None,
            dry: false,
            debug: false,
            mem_limit_gb: None,
            no_mem_limit: true,
            force: false,
            clean_intermediates: false,
        }).unwrap();

        assert!(out.join("sub-1/anat/sub-1_Chimap.nii").exists());
        assert!(out.join("sub-1/anat/sub-1_swi.nii").exists());
        assert!(out.join("sub-1/anat/sub-1_T2starmap.nii").exists());
        assert!(out.join("sub-1/anat/sub-1_R2starmap.nii").exists());
    }

    #[test]
    fn test_run_single_echo_tgv() {
        let dir = tempfile::tempdir().unwrap();
        let bids = dir.path().join("bids");
        let out = dir.path().join("out");
        testutils::create_single_echo_bids(&bids);

        super::run::execute(RunArgs {
            bids_dir: bids,
            output_dir: out.clone(),
            preset: Some(Preset::Body), // TGV preset
            config: None,
            subjects: None,
            sessions: None,
            acquisitions: None,
            runs: None,
            num_echoes: None,
            qsm_algorithm: None, // Body preset uses TGV
            unwrapping_algorithm: None,
            bf_algorithm: None,
            masking_algorithm: None,
            masking_input: Some(MaskInputArg::MagnitudeFirst),
            combine_phase: None,
            bet_fractional_intensity: None,
            bet_smoothness: None,
            bet_gradient_threshold: None,
            bet_iterations: None,
            bet_subdivisions: None,
            qsm_reference: None,
            tgv_alpha1: None,
            tgv_alpha0: None,
            mask_erosions: Some(vec![0]),
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
            tgv_iterations: Some(5), // minimal for speed
            tgv_erosions: Some(0), // 8×8×8 too small for erosion
            n_procs: Some(1),
            do_swi: false,
            do_t2starmap: false,
            do_r2starmap: false,
            inhomogeneity_correction: true,
            obliquity_threshold: None,
            mask_ops: Some(vec![
                "input:magnitude".to_string(),
                "threshold:otsu".to_string(),
            ]),
            dry: false,
            debug: false,
            mem_limit_gb: None,
            no_mem_limit: true,
            force: false,
            clean_intermediates: false,
        }).unwrap();

        assert!(out.join("sub-1/anat/sub-1_Chimap.nii").exists());
    }

    #[test]
    fn test_run_with_mask_ops() {
        let dir = tempfile::tempdir().unwrap();
        let bids = dir.path().join("bids");
        let out = dir.path().join("out");
        testutils::create_single_echo_bids(&bids);

        super::run::execute(RunArgs {
            bids_dir: bids,
            output_dir: out.clone(),
            preset: Some(Preset::Gre),
            config: None,
            subjects: None,
            sessions: None,
            acquisitions: None,
            runs: None,
            num_echoes: None,
            qsm_algorithm: Some(QsmAlgorithmArg::Tkd),
            unwrapping_algorithm: Some(UnwrapAlgorithmArg::Laplacian),
            bf_algorithm: Some(BfAlgorithmArg::Vsharp),
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
            n_procs: Some(1),
            do_swi: false,
            do_t2starmap: false,
            do_r2starmap: false,
            inhomogeneity_correction: false,
            obliquity_threshold: None,
            mask_ops: Some(vec![
                "input:magnitude".to_string(),
                "threshold:otsu".to_string(),
                "dilate:1".to_string(),
                "erode:1".to_string(),
            ]),
            dry: false,
            debug: false,
            mem_limit_gb: None,
            no_mem_limit: true,
            force: false,
            clean_intermediates: true,
        }).unwrap();

        assert!(out.join("sub-1/anat/sub-1_Chimap.nii").exists());
    }

    // --- Invert algorithms ---

    #[test]
    fn test_invert_rts() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("field.nii");
        let mask = dir.path().join("mask.nii");
        let output = dir.path().join("chi.nii");
        testutils::write_field(&input);
        testutils::write_mask(&mask);

        super::invert::execute(InvertArgs {
            input,
            mask,
            output: output.clone(),
            algorithm: QsmAlgorithmArg::Rts,
            b0_direction: vec![0.0, 0.0, 1.0],
            rts_delta: 0.15,
            rts_mu: 1e5,
            rts_tol: 0.5, // loose tolerance for speed
            rts_rho: 10.0,
            rts_max_iter: 20,
            rts_lsmr_iter: 4,
            tv_lambda: 1e-3,
            tv_rho: 0.02,
            tv_tol: 1e-3,
            tv_max_iter: 250,
            tkd_threshold: 0.15,
            tgv_iterations: 10,
            tgv_erosions: 1,
            field_strength: None,
            echo_time: None,
        }).unwrap();
        assert!(output.exists());
    }

    #[test]
    fn test_invert_tv() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("field.nii");
        let mask = dir.path().join("mask.nii");
        let output = dir.path().join("chi.nii");
        testutils::write_field(&input);
        testutils::write_mask(&mask);

        super::invert::execute(InvertArgs {
            input,
            mask,
            output: output.clone(),
            algorithm: QsmAlgorithmArg::Tv,
            b0_direction: vec![0.0, 0.0, 1.0],
            rts_delta: 0.15,
            rts_mu: 1e5,
            rts_tol: 1e-4,
            rts_rho: 10.0,
            rts_max_iter: 20,
            rts_lsmr_iter: 4,
            tv_lambda: 1e-3,
            tv_rho: 0.02,
            tv_tol: 1e-3,
            tv_max_iter: 250,
            tkd_threshold: 0.15,
            tgv_iterations: 10,
            tgv_erosions: 1,
            field_strength: None,
            echo_time: None,
        }).unwrap();
        assert!(output.exists());
    }

    #[test]
    fn test_invert_tgv() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("field.nii");
        let mask = dir.path().join("mask.nii");
        let output = dir.path().join("chi.nii");
        testutils::write_field(&input);
        testutils::write_mask(&mask);

        super::invert::execute(InvertArgs {
            input,
            mask,
            output: output.clone(),
            algorithm: QsmAlgorithmArg::Tgv,
            b0_direction: vec![0.0, 0.0, 1.0],
            rts_delta: 0.15,
            rts_mu: 1e5,
            rts_tol: 1e-4,
            rts_rho: 10.0,
            rts_max_iter: 20,
            rts_lsmr_iter: 4,
            tv_lambda: 1e-3,
            tv_rho: 0.02,
            tv_tol: 1e-3,
            tv_max_iter: 250,
            tkd_threshold: 0.15,
            tgv_iterations: 5, // minimal for speed
            tgv_erosions: 0, // 8×8×8 too small for erosion
            field_strength: Some(3.0),
            echo_time: Some(0.02),
        }).unwrap();
        assert!(output.exists());
    }

    // --- SWI with MIP ---

    #[test]
    fn test_swi_with_mip() {
        let dir = tempfile::tempdir().unwrap();
        let phase = dir.path().join("phase.nii");
        let mag = dir.path().join("mag.nii");
        let mask = dir.path().join("mask.nii");
        let output = dir.path().join("swi.nii");
        let mip = dir.path().join("mip.nii");
        testutils::write_phase(&phase);
        testutils::write_magnitude(&mag);
        testutils::write_mask(&mask);

        super::swi::execute(SwiArgs {
            phase,
            magnitude: mag,
            mask,
            output: output.clone(),
            mip: true,
            mip_output: Some(mip.clone()),
        }).unwrap();
        assert!(output.exists());
        assert!(mip.exists());
    }

    // --- Quality map with all optional inputs ---

    #[test]
    fn test_quality_map_with_magnitude_and_phase2() {
        let dir = tempfile::tempdir().unwrap();
        let phase = dir.path().join("phase.nii");
        let mag = dir.path().join("mag.nii");
        let phase2 = dir.path().join("phase2.nii");
        let output = dir.path().join("quality.nii");
        testutils::write_phase(&phase);
        testutils::write_magnitude(&mag);
        testutils::write_phase(&phase2);

        super::quality_map::execute(QualityMapArgs {
            phase,
            output: output.clone(),
            magnitude: Some(mag),
            phase2: Some(phase2),
            te1: 0.004,
            te2: 0.008,
        }).unwrap();
        assert!(output.exists());
    }

    // --- Bgremove remaining algorithms ---

    #[test]
    fn test_bgremove_lbv() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("field.nii");
        let mask = dir.path().join("mask.nii");
        let output = dir.path().join("local.nii");
        testutils::write_field(&input);
        testutils::write_mask(&mask);

        super::bgremove::execute(BgremoveArgs {
            input,
            mask,
            output: output.clone(),
            algorithm: BfAlgorithmArg::Lbv,
            b0_direction: vec![0.0, 0.0, 1.0],
            output_mask: None,
        }).unwrap();
        assert!(output.exists());
    }

    #[test]
    fn test_bgremove_ismv() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("field.nii");
        let mask = dir.path().join("mask.nii");
        let output = dir.path().join("local.nii");
        testutils::write_field(&input);
        testutils::write_mask(&mask);

        super::bgremove::execute(BgremoveArgs {
            input,
            mask,
            output: output.clone(),
            algorithm: BfAlgorithmArg::Ismv,
            b0_direction: vec![0.0, 0.0, 1.0],
            output_mask: None,
        }).unwrap();
        assert!(output.exists());
    }

    // --- Slurm command ---

    #[test]
    fn test_slurm_command() {
        let dir = tempfile::tempdir().unwrap();
        let bids = dir.path().join("bids");
        let out = dir.path().join("out");
        testutils::create_single_echo_bids(&bids);

        super::slurm::execute(SlurmArgs {
            bids_dir: bids,
            output_dir: out.clone(),
            account: "testacct".to_string(),
            partition: Some("gpu".to_string()),
            preset: Some(Preset::Gre),
            config: None,
            time: "01:00:00".to_string(),
            mem: 16,
            cpus_per_task: 2,
            submit: false,
        }).unwrap();

        assert!(out.join("slurm").exists());
    }

    #[test]
    fn test_validate_multi_session() {
        let dir = tempfile::tempdir().unwrap();
        testutils::create_multi_session_bids(dir.path());

        super::validate::execute(ValidateArgs {
            bids_dir: dir.path().to_path_buf(),
            subjects: None,
            sessions: None,
        }).unwrap();
    }
}
