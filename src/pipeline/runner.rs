use qsm_core::nifti_io::{self, NiftiData};

use crate::bids::discovery::QsmRun;
use crate::pipeline::config::*;
use crate::pipeline::phase;
use crate::error::QsmxtError;

/// Pipeline outputs for a single run.
pub struct PipelineOutputs {
    pub chi: Vec<f64>,
    pub mask: Vec<u8>,
    pub dims: (usize, usize, usize),
    pub voxel_size: (f64, f64, f64),
    pub affine: [f64; 16],
    pub swi: Option<Vec<f64>>,
    pub swi_mip: Option<Vec<f64>>,
}

/// Execute the QSM pipeline for a single run.
pub fn run_qsm_pipeline(
    qsm_run: &QsmRun,
    config: &PipelineConfig,
    progress: &dyn Fn(&str),
) -> crate::Result<PipelineOutputs> {
    // === STEP 1: Load NIfTI files ===
    progress("Loading NIfTI files");

    let mut phases: Vec<NiftiData> = Vec::new();
    let mut magnitudes: Vec<NiftiData> = Vec::new();

    for echo in &qsm_run.echoes {
        let phase = nifti_io::read_nifti_file(&echo.phase_nifti)
            .map_err(|e| QsmxtError::NiftiIo(format!("{}: {}", echo.phase_nifti.display(), e)))?;
        phases.push(phase);

        if let Some(ref mag_path) = echo.magnitude_nifti {
            let mag = nifti_io::read_nifti_file(mag_path)
                .map_err(|e| QsmxtError::NiftiIo(format!("{}: {}", mag_path.display(), e)))?;
            magnitudes.push(mag);
        }
    }

    if phases.is_empty() {
        return Err(QsmxtError::NoPhaseFiles {
            subject: qsm_run.key.subject.clone(),
            session: qsm_run
                .key
                .session
                .as_deref()
                .unwrap_or("")
                .to_string(),
        });
    }

    let (nx, ny, nz) = phases[0].dims;
    let (vsx, vsy, vsz) = phases[0].voxel_size;
    let affine = phases[0].affine;
    let bdir = qsm_run.b0_dir;

    // === STEP 2: Scale phase to [-pi, pi] ===
    progress("Scaling phase");
    for p in &mut phases {
        phase::scale_phase_to_pi(&mut p.data);
    }

    // === STEP 3: Create mask ===
    progress("Creating mask");
    let mask = create_mask(&phases, &magnitudes, nx, ny, nz, vsx, vsy, vsz, config)?;

    // Apply erosions
    let mut working_mask = mask.clone();
    for &erosion in &config.mask_erosions {
        if erosion > 0 {
            working_mask = phase::erode_mask(&working_mask, nx, ny, nz, erosion);
        }
    }

    // === Optional: SWI ===
    let (swi, swi_mip) = if config.do_swi && !magnitudes.is_empty() {
        progress("Computing SWI");
        let swi_result = qsm_core::swi::calculate_swi_default(
            &phases[0].data,
            &magnitudes[0].data,
            &working_mask,
            nx, ny, nz,
            vsx, vsy, vsz,
        );
        let mip = qsm_core::swi::create_mip_default(&swi_result, nx, ny, nz);
        (Some(swi_result), Some(mip))
    } else {
        (None, None)
    };

    // === STEP 4-6: QSM reconstruction ===
    let chi = if config.qsm_algorithm == QsmAlgorithm::Tgv {
        run_tgv_pipeline(&phases, &working_mask, nx, ny, nz, vsx, vsy, vsz, bdir, qsm_run, config, progress)?
    } else {
        run_standard_pipeline(&phases, &magnitudes, &working_mask, nx, ny, nz, vsx, vsy, vsz, bdir, qsm_run, config, progress)?
    };

    // === STEP 7: Reference ===
    let chi_final = apply_reference(&chi, &working_mask, config);

    Ok(PipelineOutputs {
        chi: chi_final,
        mask: working_mask,
        dims: (nx, ny, nz),
        voxel_size: (vsx, vsy, vsz),
        affine,
        swi,
        swi_mip,
    })
}

fn create_mask(
    phases: &[NiftiData],
    magnitudes: &[NiftiData],
    nx: usize, ny: usize, nz: usize,
    vsx: f64, vsy: f64, vsz: f64,
    config: &PipelineConfig,
) -> crate::Result<Vec<u8>> {
    match config.masking_algorithm {
        MaskingAlgorithm::Bet => {
            if magnitudes.is_empty() {
                return Err(QsmxtError::Config(
                    "BET masking requires magnitude images".to_string(),
                ));
            }
            let mag_data = if magnitudes.len() > 1 {
                let refs: Vec<&[f64]> = magnitudes.iter().map(|m| m.data.as_slice()).collect();
                phase::rss_combine(&refs)
            } else {
                magnitudes[0].data.clone()
            };
            Ok(qsm_core::bet::run_bet(
                &mag_data,
                nx, ny, nz,
                vsx, vsy, vsz,
                config.bet_fractional_intensity,
                0.0,  // smoothness
                0.0,  // gradient_threshold
                1000, // iterations
                4,    // subdivisions
            ))
        }
        MaskingAlgorithm::Threshold => {
            let input_data = match config.masking_input {
                MaskingInput::Magnitude if !magnitudes.is_empty() => {
                    magnitudes[0].data.clone()
                }
                MaskingInput::Phase | MaskingInput::Magnitude => {
                    // Use absolute phase if magnitude not available
                    phases[0].data.iter().map(|v| v.abs()).collect()
                }
            };
            let threshold = qsm_core::utils::otsu_threshold(&input_data, 256);
            Ok(input_data
                .iter()
                .map(|&v| if v > threshold { 1u8 } else { 0u8 })
                .collect())
        }
    }
}

/// Standard pipeline: unwrap → BG removal → dipole inversion
fn run_standard_pipeline(
    phases: &[NiftiData],
    magnitudes: &[NiftiData],
    mask: &[u8],
    nx: usize, ny: usize, nz: usize,
    vsx: f64, vsy: f64, vsz: f64,
    bdir: (f64, f64, f64),
    qsm_run: &QsmRun,
    config: &PipelineConfig,
    progress: &dyn Fn(&str),
) -> crate::Result<Vec<f64>> {
    // === Phase unwrapping / echo combination ===
    progress("Phase unwrapping / echo combination");

    let (field_ppm, bg_mask) = if phases.len() > 1 && config.combine_phase {
        // Multi-echo MCPC-3D-S combination
        let phase_slices: Vec<&[f64]> = phases.iter().map(|p| p.data.as_slice()).collect();
        let mag_slices: Vec<&[f64]> = magnitudes.iter().map(|m| m.data.as_slice()).collect();

        let (b0_hz, _offset, _corrected) = qsm_core::utils::mcpc3ds_b0_pipeline(
            &phase_slices,
            &mag_slices,
            &qsm_run.echo_times,
            mask,
            [4.0, 4.0, 4.0],
            qsm_core::utils::B0WeightType::PhaseSNR,
            nx, ny, nz,
        );

        let field = phase::hz_to_ppm(&b0_hz, qsm_run.magnetic_field_strength);
        (field, mask.to_vec())
    } else if phases.len() > 1 && !config.combine_phase {
        // Independent unwrapping per echo + linear fit
        let mut unwrapped: Vec<Vec<f64>> = Vec::new();
        for p in phases {
            let uw = unwrap_phase(&p.data, mask, nx, ny, nz, vsx, vsy, vsz, magnitudes, config);
            unwrapped.push(uw);
        }

        let uw_refs: Vec<&[f64]> = unwrapped.iter().map(|u| u.as_slice()).collect();
        let mag_refs: Vec<&[f64]> = magnitudes.iter().map(|m| m.data.as_slice()).collect();

        let fit = qsm_core::utils::multi_echo_linear_fit(
            &uw_refs,
            &mag_refs,
            &qsm_run.echo_times,
            mask,
            true,
            90.0,
        );

        let field = phase::rads_to_ppm(&fit.field, qsm_run.magnetic_field_strength);
        (field, mask.to_vec())
    } else {
        // Single echo unwrap
        let unwrapped = unwrap_phase(&phases[0].data, mask, nx, ny, nz, vsx, vsy, vsz, magnitudes, config);
        let te = qsm_run.echo_times[0];
        // phase / TE gives rad/s
        let field_rads: Vec<f64> = unwrapped.iter().map(|&v| v / te).collect();
        let field = phase::rads_to_ppm(&field_rads, qsm_run.magnetic_field_strength);
        (field, mask.to_vec())
    };

    // === Background field removal ===
    progress("Background field removal");

    let (local_field, eroded_mask) = match config.bf_algorithm {
        Some(BfAlgorithm::Vsharp) => {
            qsm_core::bgremove::vsharp_default(&field_ppm, &bg_mask, nx, ny, nz, vsx, vsy, vsz)
        }
        Some(BfAlgorithm::Pdf) => {
            let lf = qsm_core::bgremove::pdf_default(&field_ppm, &bg_mask, nx, ny, nz, vsx, vsy, vsz);
            (lf, bg_mask.clone())
        }
        Some(BfAlgorithm::Lbv) => {
            qsm_core::bgremove::lbv_default(&field_ppm, &bg_mask, nx, ny, nz, vsx, vsy, vsz)
        }
        Some(BfAlgorithm::Ismv) => {
            qsm_core::bgremove::ismv_default(&field_ppm, &bg_mask, nx, ny, nz, vsx, vsy, vsz)
        }
        None => {
            return Err(QsmxtError::Config(
                "bf_algorithm must be set for non-TGV pipeline".to_string(),
            ));
        }
    };

    // === Dipole inversion ===
    progress("Dipole inversion");

    let chi = match config.qsm_algorithm {
        QsmAlgorithm::Rts => qsm_core::inversion::rts(
            &local_field,
            &eroded_mask,
            nx, ny, nz,
            vsx, vsy, vsz,
            bdir,
            config.rts_delta,
            config.rts_mu,
            10.0,  // rho
            config.rts_tol,
            20,    // max_iter
            4,     // lsmr_iter
        ),
        QsmAlgorithm::Tv => qsm_core::inversion::tv_admm(
            &local_field,
            &eroded_mask,
            nx, ny, nz,
            vsx, vsy, vsz,
            bdir,
            config.tv_lambda,
            0.1,   // rho
            1e-3,  // tol
            250,   // max_iter
        ),
        QsmAlgorithm::Tkd => qsm_core::inversion::tkd(
            &local_field,
            &eroded_mask,
            nx, ny, nz,
            vsx, vsy, vsz,
            bdir,
            config.tkd_threshold,
        ),
        QsmAlgorithm::Tgv => unreachable!("TGV handled separately"),
    };

    Ok(chi)
}

/// TGV single-step pipeline: takes wrapped phase directly.
fn run_tgv_pipeline(
    phases: &[NiftiData],
    mask: &[u8],
    nx: usize, ny: usize, nz: usize,
    vsx: f64, vsy: f64, vsz: f64,
    bdir: (f64, f64, f64),
    qsm_run: &QsmRun,
    config: &PipelineConfig,
    progress: &dyn Fn(&str),
) -> crate::Result<Vec<f64>> {
    progress("TGV-QSM reconstruction");

    // Use first echo's phase (already scaled to [-pi, pi])
    let phase_f32: Vec<f32> = phases[0].data.iter().map(|&v| v as f32).collect();

    let params = qsm_core::inversion::TgvParams {
        alpha0: config.tgv_alphas[1] as f32,
        alpha1: config.tgv_alphas[0] as f32,
        iterations: config.tgv_iterations,
        erosions: config.tgv_erosions,
        fieldstrength: qsm_run.magnetic_field_strength as f32,
        te: qsm_run.echo_times[0] as f32,
        ..Default::default()
    };

    let b0_f32 = (bdir.0 as f32, bdir.1 as f32, bdir.2 as f32);

    let chi_f32 = qsm_core::inversion::tgv_qsm(
        &phase_f32,
        mask,
        nx, ny, nz,
        vsx as f32, vsy as f32, vsz as f32,
        &params,
        b0_f32,
    );

    // Convert f32 to f64
    Ok(chi_f32.iter().map(|&v| v as f64).collect())
}

/// Unwrap phase using the configured algorithm.
fn unwrap_phase(
    phase: &[f64],
    mask: &[u8],
    nx: usize, ny: usize, nz: usize,
    vsx: f64, vsy: f64, vsz: f64,
    magnitudes: &[NiftiData],
    config: &PipelineConfig,
) -> Vec<f64> {
    match config.unwrapping_algorithm {
        Some(UnwrappingAlgorithm::Laplacian) | None => {
            qsm_core::unwrap::laplacian_unwrap(phase, mask, nx, ny, nz, vsx, vsy, vsz)
        }
        Some(UnwrappingAlgorithm::Romeo) => {
            let mag = if magnitudes.is_empty() {
                vec![1.0f64; phase.len()]
            } else {
                magnitudes[0].data.clone()
            };

            let weights = qsm_core::unwrap::calculate_weights_single_echo(
                phase, &mag, mask, nx, ny, nz,
            );

            let mut phase_mut = phase.to_vec();
            let mut mask_mut = mask.to_vec();
            let (si, sj, sk) = phase::mask_center_of_mass(mask, nx, ny, nz);

            qsm_core::region_grow::grow_region_unwrap(
                &mut phase_mut,
                &weights,
                &mut mask_mut,
                nx, ny, nz,
                si, sj, sk,
            );

            phase_mut
        }
    }
}

/// Apply QSM referencing (e.g., subtract mean within mask).
fn apply_reference(chi: &[f64], mask: &[u8], config: &PipelineConfig) -> Vec<f64> {
    match config.qsm_reference {
        QsmReference::Mean => {
            let mut sum = 0.0f64;
            let mut count = 0usize;
            for (i, &m) in mask.iter().enumerate() {
                if m > 0 {
                    sum += chi[i];
                    count += 1;
                }
            }
            if count == 0 {
                return chi.to_vec();
            }
            let mean = sum / count as f64;
            chi.iter()
                .zip(mask.iter())
                .map(|(&c, &m)| if m > 0 { c - mean } else { 0.0 })
                .collect()
        }
        QsmReference::None => chi
            .iter()
            .zip(mask.iter())
            .map(|(&c, &m)| if m > 0 { c } else { 0.0 })
            .collect(),
    }
}
