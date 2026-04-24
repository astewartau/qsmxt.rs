use std::path::Path;
use std::time::Instant;

use indicatif::{ProgressBar, ProgressStyle};
use qsm_core::nifti_io::{self, NiftiData};

use crate::bids::derivatives::DerivativeOutputs;
use crate::bids::discovery::QsmRun;
use crate::pipeline::config::*;
use crate::pipeline::graph::{PipelineState, RunMetadata};
use crate::pipeline::memory;
use crate::pipeline::phase;
use crate::error::QsmxtError;

/// Create an indicatif progress bar for iterative algorithms.
fn create_progress_bar(step_name: &str, total: u64) -> ProgressBar {
    let pb = ProgressBar::new(total);
    pb.set_style(
        ProgressStyle::with_template(&format!(
            "  {{spinner:.green}} {} [{{bar:30.cyan/dim}}] {{pos}}/{{len}} ({{percent}}%) | {{elapsed_precise}} elapsed | RSS: {{msg}}",
            step_name
        ))
        .unwrap()
        .progress_chars("━╸─"),
    );
    pb.set_message("...");
    pb
}

/// Create a progress callback that drives an indicatif progress bar.
#[allow(clippy::type_complexity)]
fn iter_progress_bar(step_name: &str) -> (Box<dyn FnMut(usize, usize) + '_>, Option<ProgressBar>) {
    let pb: std::cell::RefCell<Option<ProgressBar>> = std::cell::RefCell::new(None);
    let name = step_name.to_string();
    let cb = Box::new(move |current: usize, total: usize| {
        let mut pb_ref = pb.borrow_mut();
        if pb_ref.is_none() && total > 0 {
            *pb_ref = Some(create_progress_bar(&name, total as u64));
        }
        if let Some(ref bar) = *pb_ref {
            bar.set_position(current as u64);
            // Update memory info occasionally (reading /proc is cheap but not free)
            if current == 1 || current == total || current.is_multiple_of(10) {
                let rss = memory::process_rss_bytes();
                if rss > 0 {
                    bar.set_message(memory::format_bytes(rss));
                }
            }
            if current == total {
                bar.finish_and_clear();
            }
        }
    });
    (cb, None)
}

/// Log step completion with timing.
fn log_step_done(step_name: &str, start: Instant) {
    let elapsed = start.elapsed();
    let secs = elapsed.as_secs_f64();
    let rss = memory::process_rss_bytes();
    if rss > 0 {
        log::info!(
            "{} complete ({:.1}s, RSS: {})",
            step_name, secs, memory::format_bytes(rss),
        );
    } else {
        log::info!("{} complete ({:.1}s)", step_name, secs);
    }
}

/// Helper: save a f64 volume to NIfTI using metadata from RunMetadata.
fn save_volume(path: &Path, data: &[f64], meta: &RunMetadata) -> crate::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    nifti_io::save_nifti_to_file(path, data, meta.dims, meta.voxel_size, &meta.affine)
        .map_err(QsmxtError::NiftiIo)
}

/// Helper: save a u8 mask as f64 NIfTI.
fn save_mask(path: &Path, mask: &[u8], meta: &RunMetadata) -> crate::Result<()> {
    let data: Vec<f64> = mask.iter().map(|&m| m as f64).collect();
    save_volume(path, &data, meta)
}

/// Helper: load a f64 volume from NIfTI.
fn load_volume(path: &Path) -> crate::Result<Vec<f64>> {
    let nifti = nifti_io::read_nifti_file(path)
        .map_err(|e| QsmxtError::NiftiIo(format!("{}: {}", path.display(), e)))?;
    Ok(nifti.data)
}

/// Helper: load a u8 mask from NIfTI.
fn load_mask(path: &Path) -> crate::Result<Vec<u8>> {
    let data = load_volume(path)?;
    Ok(data.iter().map(|&v| if v > 0.5 { 1u8 } else { 0u8 }).collect())
}

/// Execute the QSM pipeline with disk caching and auto-resume.
///
/// Each step saves its output to disk and drops data from memory.
/// On re-run, completed steps with valid outputs on disk are skipped.
pub fn run_pipeline_cached(
    qsm_run: &QsmRun,
    config: &PipelineConfig,
    output: &DerivativeOutputs,
    force: bool,
    clean_intermediates: bool,
    progress: &dyn Fn(&str),
) -> crate::Result<()> {
    let state_path = output.state_path(&qsm_run.key);
    let mut state = PipelineState::load_or_create(&state_path, config, &qsm_run.key, force);

    // === STEP: Load & extract metadata ===
    let meta = if !state.is_step_cached("load") {
        let t = Instant::now();
        progress("Loading NIfTI metadata");
        let first_phase = nifti_io::read_nifti_file(&qsm_run.echoes[0].phase_nifti)
            .map_err(|e| QsmxtError::NiftiIo(format!("{}: {}", qsm_run.echoes[0].phase_nifti.display(), e)))?;

        let meta = RunMetadata {
            dims: first_phase.dims,
            voxel_size: first_phase.voxel_size,
            affine: first_phase.affine,
            n_echoes: qsm_run.echoes.len(),
            echo_times: qsm_run.echo_times.clone(),
            b0_direction: qsm_run.b0_dir,
            field_strength: qsm_run.magnetic_field_strength,
            has_magnitude: qsm_run.has_magnitude,
        };
        log::info!(
            "Volume: {}x{}x{}, {:.1}mm iso, {} echoes, B0={:.1}T, TEs={:?}s",
            meta.dims.0, meta.dims.1, meta.dims.2,
            meta.voxel_size.0, meta.n_echoes, meta.field_strength, meta.echo_times,
        );
        state.run_metadata = Some(meta.clone());
        state.mark_completed("load", vec![]);
        state.save(&state_path)?;
        log_step_done("Load", t);
        meta
    } else {
        log::info!("Skipping load (cached)");
        state.run_metadata.clone().ok_or_else(|| {
            QsmxtError::Config("Cached state missing run metadata".to_string())
        })?
    };

    let (nx, ny, nz) = meta.dims;
    let (vsx, vsy, vsz) = meta.voxel_size;

    // === STEP: Scale phase & save ===
    if !state.is_step_cached("scale_phase") {
        let t = Instant::now();
        progress("Scaling phase + saving magnitudes");
        let mut phase_paths = Vec::new();
        for (i, echo) in qsm_run.echoes.iter().enumerate() {
            let mut phase_nifti = nifti_io::read_nifti_file(&echo.phase_nifti)
                .map_err(|e| QsmxtError::NiftiIo(format!("{}: {}", echo.phase_nifti.display(), e)))?;
            phase::scale_phase_to_pi(&mut phase_nifti.data);
            let out_path = output.phase_scaled_path(&qsm_run.key, i + 1);
            save_volume(&out_path, &phase_nifti.data, &meta)?;
            phase_paths.push(out_path);
            // phase_nifti dropped here
        }
        // Also save magnitude files to output dir (so later steps can load from there)
        let mut mag_paths = Vec::new();
        if config.inhomogeneity_correction && qsm_run.has_magnitude {
            progress("Applying inhomogeneity correction");
            for (i, echo) in qsm_run.echoes.iter().enumerate() {
                if let Some(ref mag_path) = echo.magnitude_nifti {
                    let mag_nifti = nifti_io::read_nifti_file(mag_path)
                        .map_err(|e| QsmxtError::NiftiIo(format!("{}: {}", mag_path.display(), e)))?;
                    let corrected = qsm_core::utils::makehomogeneous(
                        &mag_nifti.data, nx, ny, nz, vsx, vsy, vsz, 7.0, 3,
                    );
                    let out_path = output.mag_path(&qsm_run.key, i + 1);
                    save_volume(&out_path, &corrected, &meta)?;
                    mag_paths.push(out_path);
                }
            }
        } else {
            for (i, echo) in qsm_run.echoes.iter().enumerate() {
                if let Some(ref mag_path) = echo.magnitude_nifti {
                    let out_path = output.mag_path(&qsm_run.key, i + 1);
                    // Copy or symlink magnitude
                    if let Some(parent) = out_path.parent() {
                        std::fs::create_dir_all(parent)?;
                    }
                    std::fs::copy(mag_path, &out_path)?;
                    mag_paths.push(out_path);
                }
            }
        }
        let mut all_paths = phase_paths;
        all_paths.extend(mag_paths);
        state.mark_completed("scale_phase", all_paths);
        state.save(&state_path)?;
        log_step_done("Scale phase", t);
    } else {
        log::info!("Skipping scale_phase (cached)");
    }

    // === STEP: Create mask ===
    let mask_path = output.mask_path(&qsm_run.key);
    if !state.is_step_cached("mask") {
        let t = Instant::now();
        log::info!("Creating mask ({} ops: {})",
            config.mask_ops.len(),
            config.mask_ops.iter().map(|op| format!("{}", op)).collect::<Vec<_>>().join(" → "),
        );
        progress("Creating mask");
        // Load scaled phases and magnitudes from disk
        let mut phases: Vec<NiftiData> = Vec::new();
        let mut magnitudes: Vec<NiftiData> = Vec::new();
        for i in 0..meta.n_echoes {
            let p_path = output.phase_scaled_path(&qsm_run.key, i + 1);
            if p_path.exists() {
                let p = nifti_io::read_nifti_file(&p_path)
                    .map_err(|e| QsmxtError::NiftiIo(format!("{}: {}", p_path.display(), e)))?;
                phases.push(p);
            }
            let m_path = output.mag_path(&qsm_run.key, i + 1);
            if m_path.exists() {
                let m = nifti_io::read_nifti_file(&m_path)
                    .map_err(|e| QsmxtError::NiftiIo(format!("{}: {}", m_path.display(), e)))?;
                magnitudes.push(m);
            }
        }

        let working_mask = if !config.mask_ops.is_empty() {
            build_mask_from_ops(&config.mask_ops, &phases, &magnitudes, nx, ny, nz, vsx, vsy, vsz, &meta.echo_times)?
        } else {
            let mask = create_mask(&phases, &magnitudes, nx, ny, nz, vsx, vsy, vsz, config, &meta.echo_times)?;
            let mut working = mask;
            for &erosion in &config.mask_erosions {
                if erosion > 0 {
                    working = phase::erode_mask(&working, nx, ny, nz, erosion);
                }
            }
            working
        };

        save_mask(&mask_path, &working_mask, &meta)?;
        state.mark_completed("mask", vec![mask_path.clone()]);
        state.save(&state_path)?;
        log_step_done("Mask creation", t);
        // phases, magnitudes, working_mask dropped here
    } else {
        log::info!("Skipping mask (cached)");
    }

    // === STEP: SWI (optional) ===
    if config.do_swi && meta.has_magnitude {
        let swi_path = output.swi_path(&qsm_run.key);
        let mip_path = output.swi_mip_path(&qsm_run.key);
        if !state.is_step_cached("swi") {
            let t = Instant::now();
            log::info!("Computing SWI (Laplacian unwrap + CLEAR-SWI + MIP)");
            progress("Computing SWI");
            let phase_data = load_volume(&output.phase_scaled_path(&qsm_run.key, 1))?;
            let mag_data = load_volume(&output.mag_path(&qsm_run.key, 1))?;
            let mask = load_mask(&mask_path)?;

            let unwrapped = qsm_core::unwrap::laplacian_unwrap(
                &phase_data, &mask, nx, ny, nz, vsx, vsy, vsz,
            );
            let swi = qsm_core::swi::calculate_swi_default(
                &unwrapped, &mag_data, &mask, nx, ny, nz, vsx, vsy, vsz,
            );
            let mip = qsm_core::swi::create_mip_default(&swi, nx, ny, nz);

            save_volume(&swi_path, &swi, &meta)?;
            save_volume(&mip_path, &mip, &meta)?;
            state.mark_completed("swi", vec![swi_path, mip_path]);
            state.save(&state_path)?;
            log_step_done("SWI", t);
        } else {
            log::info!("Skipping swi (cached)");
        }
    }

    // === STEP: T2*/R2* (optional) ===
    if (config.do_t2starmap || config.do_r2starmap) && meta.n_echoes >= 3 && meta.has_magnitude {
        if !state.is_step_cached("t2star_r2star") {
            let t = Instant::now();
            log::info!("Computing R2*/T2* maps (ARLO, {} echoes)", meta.n_echoes);
            progress("Computing R2*/T2* maps");
            let mask = load_mask(&mask_path)?;
            let n_voxels = nx * ny * nz;

            let mut interleaved = vec![0.0f64; n_voxels * meta.n_echoes];
            for i in 0..meta.n_echoes {
                let mag_data = load_volume(&output.mag_path(&qsm_run.key, i + 1))?;
                for vox in 0..n_voxels {
                    interleaved[vox * meta.n_echoes + i] = mag_data[vox];
                }
                // mag_data dropped each iteration
            }

            let (r2star_map, _s0) = qsm_core::utils::r2star_arlo(
                &interleaved, &mask, &meta.echo_times, nx, ny, nz,
            );
            drop(interleaved); // free immediately

            let mut paths = Vec::new();
            if config.do_r2starmap {
                let p = output.r2star_path(&qsm_run.key);
                save_volume(&p, &r2star_map, &meta)?;
                paths.push(p);
            }
            if config.do_t2starmap {
                let t2star: Vec<f64> = r2star_map.iter().zip(mask.iter())
                    .map(|(&r2, &m)| if m > 0 && r2 > 0.0 { 1.0 / r2 } else { 0.0 })
                    .collect();
                let p = output.t2star_path(&qsm_run.key);
                save_volume(&p, &t2star, &meta)?;
                paths.push(p);
            }
            state.mark_completed("t2star_r2star", paths);
            state.save(&state_path)?;
            log_step_done("T2*/R2* mapping", t);
        } else {
            log::info!("Skipping t2star_r2star (cached)");
        }
    }

    // === STEP: QSM reconstruction ===
    // Branch: TGV (single step) vs Standard (unwrap → bgremove → invert)

    if config.qsm_algorithm == QsmAlgorithm::Tgv {
        // TGV single-step
        let chi_raw_path = output.chi_raw_path(&qsm_run.key);
        if !state.is_step_cached("tgv") {
            let t = Instant::now();
            log::info!(
                "TGV-QSM (iterations={}, alphas=[{}, {}], erosions={}, TE={:.3}ms, B0={:.1}T)",
                config.tgv_iterations, config.tgv_alphas[0], config.tgv_alphas[1],
                config.tgv_erosions, meta.echo_times[0] * 1000.0, meta.field_strength,
            );
            progress("TGV-QSM reconstruction");
            let phase_data = load_volume(&output.phase_scaled_path(&qsm_run.key, 1))?;
            let mask = load_mask(&mask_path)?;
            let bdir = meta.b0_direction;

            let phase_f32: Vec<f32> = phase_data.iter().map(|&v| v as f32).collect();
            drop(phase_data);

            let params = qsm_core::inversion::TgvParams {
                alpha0: config.tgv_alphas[1] as f32,
                alpha1: config.tgv_alphas[0] as f32,
                iterations: config.tgv_iterations,
                erosions: config.tgv_erosions,
                fieldstrength: meta.field_strength as f32,
                te: meta.echo_times[0] as f32,
                ..Default::default()
            };
            let b0_f32 = (bdir.0 as f32, bdir.1 as f32, bdir.2 as f32);
            let tgv_pb: std::cell::RefCell<Option<ProgressBar>> = std::cell::RefCell::new(None);
            let chi_f32 = qsm_core::inversion::tgv_qsm_with_progress(
                &phase_f32, &mask, nx, ny, nz,
                vsx as f32, vsy as f32, vsz as f32, &params, b0_f32,
                |current: usize, total: usize| {
                    let mut pb_ref = tgv_pb.borrow_mut();
                    if pb_ref.is_none() && total > 0 {
                        *pb_ref = Some(create_progress_bar("TGV", total as u64));
                    }
                    if let Some(ref bar) = *pb_ref {
                        bar.set_position(current as u64);
                        if current == 1 || current == total || current.is_multiple_of(10) {
                            let rss = memory::process_rss_bytes();
                            if rss > 0 { bar.set_message(memory::format_bytes(rss)); }
                        }
                        if current == total { bar.finish_and_clear(); }
                    }
                },
            );
            let chi: Vec<f64> = chi_f32.iter().map(|&v| v as f64).collect();

            save_volume(&chi_raw_path, &chi, &meta)?;
            state.mark_completed("tgv", vec![chi_raw_path.clone()]);
            state.save(&state_path)?;
            log_step_done("TGV-QSM", t);
        } else {
            log::info!("Skipping tgv (cached)");
        }
    } else {
        // Standard path: unwrap → bgremove → invert

        // --- Unwrap ---
        let field_path = output.field_ppm_path(&qsm_run.key);
        if !state.is_step_cached("unwrap") {
            let t = Instant::now();
            let unwrap_name = config.unwrapping_algorithm.map(|a| format!("{}", a)).unwrap_or("laplacian".to_string());
            if meta.n_echoes > 1 && config.combine_phase {
                log::info!("Phase combination (MCPC-3D-S, {} echoes, weighted B0)", meta.n_echoes);
            } else if meta.n_echoes > 1 {
                log::info!("Phase unwrapping ({}, {} echoes, linear fit)", unwrap_name, meta.n_echoes);
            } else {
                log::info!("Phase unwrapping ({}, single echo)", unwrap_name);
            }
            progress("Phase unwrapping / echo combination");
            // Load all scaled phases + magnitudes
            let mut phases: Vec<NiftiData> = Vec::new();
            let mut magnitudes: Vec<NiftiData> = Vec::new();
            for i in 0..meta.n_echoes {
                let p = nifti_io::read_nifti_file(&output.phase_scaled_path(&qsm_run.key, i + 1))
                    .map_err(|e| QsmxtError::NiftiIo(format!("echo {}: {}", i + 1, e)))?;
                phases.push(p);
                let m_path = output.mag_path(&qsm_run.key, i + 1);
                if m_path.exists() {
                    let m = nifti_io::read_nifti_file(&m_path)
                        .map_err(|e| QsmxtError::NiftiIo(format!("mag echo {}: {}", i + 1, e)))?;
                    magnitudes.push(m);
                }
            }
            let mask = load_mask(&mask_path)?;

            // mcpc3ds_b0_pipeline expects echo times in milliseconds
            let echo_times_ms: Vec<f64> = meta.echo_times.iter().map(|&t| t * 1000.0).collect();

            let field_ppm = if phases.len() > 1 && config.combine_phase {
                let phase_slices: Vec<&[f64]> = phases.iter().map(|p| p.data.as_slice()).collect();
                let mag_slices: Vec<&[f64]> = magnitudes.iter().map(|m| m.data.as_slice()).collect();
                let (b0_hz, _, _) = qsm_core::utils::mcpc3ds_b0_pipeline(
                    &phase_slices, &mag_slices, &echo_times_ms, &mask,
                    [4.0, 4.0, 4.0], qsm_core::utils::B0WeightType::PhaseSNR, nx, ny, nz,
                );
                phase::hz_to_ppm(&b0_hz, meta.field_strength)
            } else if phases.len() > 1 {
                let mut unwrapped: Vec<Vec<f64>> = Vec::new();
                for p in &phases {
                    let uw = unwrap_phase(&p.data, &mask, nx, ny, nz, vsx, vsy, vsz, &magnitudes, config);
                    unwrapped.push(uw);
                }
                let uw_refs: Vec<&[f64]> = unwrapped.iter().map(|u| u.as_slice()).collect();
                let mag_refs: Vec<&[f64]> = magnitudes.iter().map(|m| m.data.as_slice()).collect();
                let fit = qsm_core::utils::multi_echo_linear_fit(
                    &uw_refs, &mag_refs, &meta.echo_times, &mask, true, 90.0,
                );
                phase::rads_to_ppm(&fit.field, meta.field_strength)
            } else {
                let unwrapped = unwrap_phase(&phases[0].data, &mask, nx, ny, nz, vsx, vsy, vsz, &magnitudes, config);
                let te = meta.echo_times[0];
                let field_rads: Vec<f64> = unwrapped.iter().map(|&v| v / te).collect();
                phase::rads_to_ppm(&field_rads, meta.field_strength)
            };

            save_volume(&field_path, &field_ppm, &meta)?;
            state.mark_completed("unwrap", vec![field_path.clone()]);
            state.save(&state_path)?;
            log_step_done("Phase unwrapping", t);
            // phases, magnitudes, field_ppm all dropped
        } else {
            log::info!("Skipping unwrap (cached)");
        }

        // --- Background removal ---
        let local_field_path = output.local_field_path(&qsm_run.key);
        let bg_mask_path = output.bg_mask_path(&qsm_run.key);
        if !state.is_step_cached("bgremove") {
            let t = Instant::now();
            let bf_name = config.bf_algorithm.map(|a| format!("{}", a)).unwrap_or("none".to_string());
            progress("Background field removal");
            let field_ppm = load_volume(&field_path)?;
            let mask = load_mask(&mask_path)?;
            let bdir = meta.b0_direction;

            let (local_field, eroded_mask) = match config.bf_algorithm {
                Some(BfAlgorithm::Vsharp) => {
                    let min_vox = vsx.min(vsy).min(vsz);
                    let max_vox = vsx.max(vsy).max(vsz);
                    let mut radii = Vec::new();
                    let mut r = 18.0 * min_vox;
                    while r >= 2.0 * max_vox { radii.push(r); r -= 2.0 * max_vox; }
                    log::info!("Background removal (V-SHARP, {} radii, threshold=0.05)", radii.len());
                    let (prog, _) = iter_progress_bar("V-SHARP");
                    qsm_core::bgremove::vsharp_with_progress(
                        &field_ppm, &mask, nx, ny, nz, vsx, vsy, vsz,
                        &radii, 0.05, prog,
                    )
                }
                Some(BfAlgorithm::Pdf) => {
                    // Cap at 100 iterations (sufficient for convergence, validated in QSM.rs CI)
                    let max_iter = 100;
                    log::info!("Background removal (PDF, tol=1e-5, max_iter={}, B0=[{:.2},{:.2},{:.2}])",
                        max_iter, bdir.0, bdir.1, bdir.2);
                    let (prog, _) = iter_progress_bar("PDF");
                    let lf = qsm_core::bgremove::pdf_with_progress(
                        &field_ppm, &mask, nx, ny, nz, vsx, vsy, vsz,
                        bdir, 1e-5, max_iter, prog,
                    );
                    (lf, mask.clone())
                }
                Some(BfAlgorithm::Lbv) => {
                    let max_iter = (3 * nx.max(ny).max(nz)).max(500);
                    log::info!("Background removal (LBV, tol=1e-6, max_iter={})", max_iter);
                    let (prog, _) = iter_progress_bar("LBV");
                    qsm_core::bgremove::lbv_with_progress(
                        &field_ppm, &mask, nx, ny, nz, vsx, vsy, vsz,
                        1e-6, max_iter, prog,
                    )
                }
                Some(BfAlgorithm::Ismv) => {
                    let radius = 2.0 * vsx.max(vsy).max(vsz);
                    log::info!("Background removal (iSMV, radius={:.1}, tol=1e-3, max_iter=500)", radius);
                    let (prog, _) = iter_progress_bar("iSMV");
                    qsm_core::bgremove::ismv_with_progress(
                        &field_ppm, &mask, nx, ny, nz, vsx, vsy, vsz,
                        radius, 1e-3, 500, prog,
                    )
                }
                None => {
                    return Err(QsmxtError::Config("bf_algorithm must be set for non-TGV pipeline".to_string()));
                }
            };

            save_volume(&local_field_path, &local_field, &meta)?;
            save_mask(&bg_mask_path, &eroded_mask, &meta)?;
            state.mark_completed("bgremove", vec![local_field_path.clone(), bg_mask_path.clone()]);
            state.save(&state_path)?;
            log_step_done(&format!("Background removal ({})", bf_name), t);
        } else {
            log::info!("Skipping bgremove (cached)");
        }

        // --- Dipole inversion ---
        let chi_raw_path = output.chi_raw_path(&qsm_run.key);
        if !state.is_step_cached("invert") {
            let t = Instant::now();
            progress("Dipole inversion");
            let local_field = load_volume(&local_field_path)?;
            let eroded_mask = load_mask(&bg_mask_path)?;
            let bdir = meta.b0_direction;

            let chi = match config.qsm_algorithm {
                QsmAlgorithm::Rts => {
                    log::info!("Dipole inversion (RTS, delta={}, mu={:.0e}, tol={:.0e}, max_iter=20)",
                        config.rts_delta, config.rts_mu, config.rts_tol);
                    let (prog, _) = iter_progress_bar("RTS");
                    qsm_core::inversion::rts_with_progress(
                        &local_field, &eroded_mask, nx, ny, nz, vsx, vsy, vsz,
                        bdir, config.rts_delta, config.rts_mu, 10.0, config.rts_tol, 20, 4,
                        prog,
                    )
                }
                QsmAlgorithm::Tv => {
                    log::info!("Dipole inversion (TV-ADMM, lambda={:.0e}, max_iter=250)", config.tv_lambda);
                    let (prog, _) = iter_progress_bar("TV");
                    qsm_core::inversion::tv_admm_with_progress(
                        &local_field, &eroded_mask, nx, ny, nz, vsx, vsy, vsz,
                        bdir, config.tv_lambda, 0.1, 1e-3, 250,
                        prog,
                    )
                }
                QsmAlgorithm::Tkd => {
                    log::info!("Dipole inversion (TKD, threshold={})", config.tkd_threshold);
                    qsm_core::inversion::tkd(
                        &local_field, &eroded_mask, nx, ny, nz, vsx, vsy, vsz,
                        bdir, config.tkd_threshold,
                    )
                }
                QsmAlgorithm::Tgv => unreachable!("TGV handled separately"),
            };

            save_volume(&chi_raw_path, &chi, &meta)?;
            state.mark_completed("invert", vec![chi_raw_path.clone()]);
            state.save(&state_path)?;
            log_step_done(&format!("Dipole inversion ({})", config.qsm_algorithm), t);
        } else {
            log::info!("Skipping invert (cached)");
        }
    }

    // === STEP: Reference ===
    let qsm_path = output.qsm_path(&qsm_run.key);
    if !state.is_step_cached("reference") {
        let t = Instant::now();
        log::info!("QSM referencing ({})", config.qsm_reference);
        progress("Referencing QSM");
        let chi_raw_path = output.chi_raw_path(&qsm_run.key);
        let chi = load_volume(&chi_raw_path)?;
        let mask = load_mask(&mask_path)?;

        let chi_final = apply_reference(&chi, &mask, config);

        save_volume(&qsm_path, &chi_final, &meta)?;
        state.mark_completed("reference", vec![qsm_path.clone()]);
        state.save(&state_path)?;
        log_step_done("QSM referencing", t);
    } else {
        log::info!("Skipping reference (cached)");
    }

    // === Done ===
    state.mark_run_complete();
    state.save(&state_path)?;

    if clean_intermediates {
        crate::pipeline::graph::clean_intermediates(&state, &output.output_dir, &qsm_run.key);
    }

    Ok(())
}


/// Resolve masking input data based on the MaskingInput type.
fn resolve_masking_input(
    source: &MaskingInput,
    phases: &[NiftiData],
    magnitudes: &[NiftiData],
    nx: usize, ny: usize, nz: usize,
    echo_times: &[f64],
) -> Vec<f64> {
    let n_voxels = nx * ny * nz;
    match source {
        MaskingInput::MagnitudeFirst if !magnitudes.is_empty() => {
            magnitudes[0].data.clone()
        }
        MaskingInput::Magnitude if !magnitudes.is_empty() => {
            if magnitudes.len() > 1 {
                let refs: Vec<&[f64]> = magnitudes.iter().map(|m| m.data.as_slice()).collect();
                phase::rss_combine(&refs)
            } else {
                magnitudes[0].data.clone()
            }
        }
        MaskingInput::MagnitudeLast if !magnitudes.is_empty() => {
            magnitudes.last().unwrap().data.clone()
        }
        MaskingInput::PhaseQuality => {
            let all_ones = vec![1u8; n_voxels];
            let mag = if !magnitudes.is_empty() {
                magnitudes[0].data.clone()
            } else {
                vec![1.0f64; n_voxels]
            };
            if phases.len() >= 2 && echo_times.len() >= 2 {
                qsm_core::unwrap::voxel_quality_romeo(
                    &phases[0].data, &mag,
                    Some(&phases[1].data),
                    echo_times[0], echo_times[1],
                    &all_ones, nx, ny, nz,
                )
            } else {
                qsm_core::unwrap::voxel_quality_romeo(
                    &phases[0].data, &mag,
                    None,
                    echo_times.first().copied().unwrap_or(0.02),
                    0.0, &all_ones, nx, ny, nz,
                )
            }
        }
        _ => {
            // Magnitude fallback when no magnitude available
            vec![0.0; n_voxels]
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn create_mask(
    phases: &[NiftiData],
    magnitudes: &[NiftiData],
    nx: usize, ny: usize, nz: usize,
    vsx: f64, vsy: f64, vsz: f64,
    config: &PipelineConfig,
    echo_times: &[f64],
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
                1.0,  // smoothness
                0.0,  // gradient_threshold
                1000, // iterations
                4,    // subdivisions
            ))
        }
        MaskingAlgorithm::Threshold => {
            if magnitudes.is_empty() && config.masking_input != MaskingInput::PhaseQuality {
                return Err(QsmxtError::Config(
                    "Threshold masking requires magnitude images (or use --masking-input phase-quality)".to_string(),
                ));
            }
            let input_data = resolve_masking_input(
                &config.masking_input, phases, magnitudes, nx, ny, nz, echo_times,
            );
            let threshold = qsm_core::utils::otsu_threshold(&input_data, 256);
            Ok(input_data
                .iter()
                .map(|&v| if v > threshold { 1u8 } else { 0u8 })
                .collect())
        }
    }
}

/// Build mask from an ordered sequence of mask operations.
#[allow(clippy::too_many_arguments)]
fn build_mask_from_ops(
    ops: &[MaskOp],
    phases: &[NiftiData],
    magnitudes: &[NiftiData],
    nx: usize, ny: usize, nz: usize,
    vsx: f64, vsy: f64, vsz: f64,
    echo_times: &[f64],
) -> crate::Result<Vec<u8>> {
    use crate::pipeline::config::{MaskOp, MaskThresholdMethod};

    let n_voxels = nx * ny * nz;
    let mut mask = vec![1u8; n_voxels];
    // Default input data: magnitude if available, else absolute phase
    let mut input_data: Vec<f64> = if !magnitudes.is_empty() {
        magnitudes[0].data.clone()
    } else if !phases.is_empty() {
        phases[0].data.iter().map(|v| v.abs()).collect()
    } else {
        vec![0.0; n_voxels]
    };

    for op in ops {
        match op {
            MaskOp::Input { source } => {
                input_data = resolve_masking_input(
                    source, phases, magnitudes, nx, ny, nz, echo_times,
                );
            }
            MaskOp::Threshold { method, value } => {
                let threshold = match method {
                    MaskThresholdMethod::Otsu => {
                        qsm_core::utils::otsu_threshold(&input_data, 256)
                    }
                    MaskThresholdMethod::Fixed => value.unwrap_or(0.5),
                    MaskThresholdMethod::Percentile => {
                        let pct = value.unwrap_or(75.0) / 100.0;
                        let mut sorted: Vec<f64> = input_data.iter()
                            .filter(|v| v.is_finite() && **v > 0.0)
                            .copied().collect();
                        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
                        if sorted.is_empty() { 0.0 }
                        else {
                            let idx = ((sorted.len() as f64 * pct) as usize).min(sorted.len() - 1);
                            sorted[idx]
                        }
                    }
                };
                mask = input_data.iter()
                    .map(|&v| if v > threshold { 1u8 } else { 0u8 })
                    .collect();
            }
            MaskOp::Bet { fractional_intensity } => {
                if magnitudes.is_empty() {
                    return Err(QsmxtError::Config("BET requires magnitude images".to_string()));
                }
                let mag_data = if magnitudes.len() > 1 {
                    let refs: Vec<&[f64]> = magnitudes.iter().map(|m| m.data.as_slice()).collect();
                    phase::rss_combine(&refs)
                } else {
                    magnitudes[0].data.clone()
                };
                mask = qsm_core::bet::run_bet(
                    &mag_data, nx, ny, nz, vsx, vsy, vsz,
                    *fractional_intensity, 1.0, 0.0, 1000, 4,
                );
            }
            MaskOp::Erode { iterations } => {
                mask = phase::erode_mask(&mask, nx, ny, nz, *iterations);
            }
            MaskOp::Dilate { iterations } => {
                mask = phase::dilate_mask(&mask, nx, ny, nz, *iterations);
            }
            MaskOp::Close { radius } => {
                mask = qsm_core::utils::morphological_close(&mask, nx, ny, nz, *radius as i32);
            }
            MaskOp::FillHoles { max_size } => {
                let effective_size = if *max_size == 0 { n_voxels / 20 } else { *max_size };
                mask = qsm_core::utils::fill_holes(&mask, nx, ny, nz, effective_size);
            }
            MaskOp::GaussianSmooth { sigma_mm } => {
                let sigma = *sigma_mm;
                let mask_f64: Vec<f64> = mask.iter().map(|&m| m as f64).collect();
                let smoothed = qsm_core::utils::gaussian_smooth_3d(
                    &mask_f64,
                    [sigma, sigma, sigma],
                    None, None, 3,
                    nx, ny, nz,
                );
                mask = smoothed.iter().map(|&v| if v > 0.5 { 1u8 } else { 0u8 }).collect();
            }
        }
    }

    Ok(mask)
}

/// Unwrap phase using the configured algorithm.
#[allow(clippy::too_many_arguments)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli;

    fn config_with_reference(reference: QsmReference) -> PipelineConfig {
        let mut c = PipelineConfig::from_preset(cli::Preset::Gre);
        c.qsm_reference = reference;
        c
    }

    #[test]
    fn test_apply_reference_mean_all_masked() {
        let chi = vec![1.0, 2.0, 3.0];
        let mask = vec![1u8, 1, 1];
        let config = config_with_reference(QsmReference::Mean);
        let result = apply_reference(&chi, &mask, &config);
        // Mean = 2.0, so result = [-1, 0, 1]
        assert!((result[0] - (-1.0)).abs() < 1e-10);
        assert!((result[1] - 0.0).abs() < 1e-10);
        assert!((result[2] - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_apply_reference_mean_partial_mask() {
        let chi = vec![1.0, 2.0, 3.0, 4.0];
        let mask = vec![1u8, 0, 1, 0];
        let config = config_with_reference(QsmReference::Mean);
        let result = apply_reference(&chi, &mask, &config);
        // Mean of masked = (1+3)/2 = 2.0
        assert!((result[0] - (-1.0)).abs() < 1e-10);
        assert!((result[1] - 0.0).abs() < 1e-10); // unmasked → 0
        assert!((result[2] - 1.0).abs() < 1e-10);
        assert!((result[3] - 0.0).abs() < 1e-10); // unmasked → 0
    }

    #[test]
    fn test_apply_reference_mean_empty_mask() {
        let chi = vec![1.0, 2.0, 3.0];
        let mask = vec![0u8, 0, 0];
        let config = config_with_reference(QsmReference::Mean);
        let result = apply_reference(&chi, &mask, &config);
        // Empty mask returns original chi
        assert_eq!(result, chi);
    }

    #[test]
    fn test_apply_reference_none() {
        let chi = vec![1.0, 2.0, 3.0];
        let mask = vec![1u8, 0, 1];
        let config = config_with_reference(QsmReference::None);
        let result = apply_reference(&chi, &mask, &config);
        assert!((result[0] - 1.0).abs() < 1e-10);
        assert!((result[1] - 0.0).abs() < 1e-10); // unmasked → 0
        assert!((result[2] - 3.0).abs() < 1e-10);
    }
}
