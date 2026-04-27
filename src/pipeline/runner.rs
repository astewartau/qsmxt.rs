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

    // Determine which steps are needed
    let needs_mask = config.do_qsm || config.do_swi
        || (config.do_t2starmap && meta.n_echoes >= 3 && meta.has_magnitude)
        || (config.do_r2starmap && meta.n_echoes >= 3 && meta.has_magnitude);
    let needs_phase = needs_mask || config.do_qsm;

    if !needs_phase {
        log::info!("No outputs enabled — nothing to process");
        state.mark_run_complete();
        state.save(&state_path)?;
        return Ok(());
    }

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
                        &mag_nifti.data, nx, ny, nz, vsx, vsy, vsz,
                        config.homogeneity_sigma_mm, config.homogeneity_nbox,
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

    // === STEP: Create mask (only if something needs it) ===
    let mask_path = output.mask_path(&qsm_run.key);
    if needs_mask && !state.is_step_cached("mask") {
        let t = Instant::now();
        log::info!("Creating mask ({} section(s))", config.mask_sections.len());
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

        let working_mask = build_mask_from_sections(&config.mask_sections, &phases, &magnitudes, nx, ny, nz, vsx, vsy, vsz, &meta.echo_times)?;

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
            let swi_scaling = match config.swi_scaling.as_str() {
                "negative_tanh" => qsm_core::swi::PhaseScaling::NegativeTanh,
                "positive" => qsm_core::swi::PhaseScaling::Positive,
                "negative" => qsm_core::swi::PhaseScaling::Negative,
                "triangular" => qsm_core::swi::PhaseScaling::Triangular,
                _ => qsm_core::swi::PhaseScaling::Tanh,
            };
            let swi = qsm_core::swi::calculate_swi(
                &unwrapped, &mag_data, &mask, nx, ny, nz, vsx, vsy, vsz,
                config.swi_hp_sigma, swi_scaling, config.swi_strength,
            );
            let mip = qsm_core::swi::create_mip(&swi, nx, ny, nz, config.swi_mip_window);

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

            // T2*/R2* uses uncorrected magnitude (inhomogeneity correction distorts decay)
            let mut interleaved = vec![0.0f64; n_voxels * meta.n_echoes];
            for i in 0..meta.n_echoes {
                let mag_data = if let Some(ref raw_path) = qsm_run.echoes[i].magnitude_nifti {
                    let nifti = nifti_io::read_nifti_file(raw_path)
                        .map_err(|e| QsmxtError::NiftiIo(format!("mag echo {}: {}", i + 1, e)))?;
                    nifti.data
                } else {
                    load_volume(&output.mag_path(&qsm_run.key, i + 1))?
                };
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

    // === STEP: QSM reconstruction (skipped if do_qsm is false) ===
    if !config.do_qsm {
        log::info!("QSM processing disabled — skipping reconstruction");
    }

    if config.do_qsm {
    // All modes share phase combination/unwrapping for multi-echo data.
    // Then branch: TGV (single step), QSMART (two-stage SDF+iLSQR), Standard (bgremove → invert).

    // --- Phase combination / unwrapping (shared by all modes) ---
    // TGV single-echo skips this (uses raw wrapped phase directly).
    let field_path = output.field_ppm_path(&qsm_run.key);
    let need_field = match config.qsm_algorithm {
        QsmAlgorithm::Tgv if meta.n_echoes == 1 => false,
        _ => true,
    };

    if need_field && !state.is_step_cached("unwrap") {
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
                config.mcpc3ds_sigma, qsm_core::utils::B0WeightType::PhaseSNR, nx, ny, nz,
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
                &uw_refs, &mag_refs, &meta.echo_times, &mask, true, config.linear_fit_reliability_threshold,
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
    } else if need_field {
        log::info!("Skipping unwrap (cached)");
    }

    // --- Branch: TGV / QSMART / Standard ---

    if config.qsm_algorithm == QsmAlgorithm::Tgv {
        // TGV single-step: uses combined field for multi-echo, raw phase for single-echo
        let chi_raw_path = output.chi_raw_path(&qsm_run.key);
        if !state.is_step_cached("tgv") {
            let t = Instant::now();
            log::info!(
                "TGV-QSM (iterations={}, alphas=[{}, {}], erosions={}, TE={:.3}ms, B0={:.1}T)",
                config.tgv_iterations, config.tgv_alphas[0], config.tgv_alphas[1],
                config.tgv_erosions, meta.echo_times[0] * 1000.0, meta.field_strength,
            );
            progress("TGV-QSM reconstruction");
            let mask = load_mask(&mask_path)?;
            let bdir = meta.b0_direction;

            // For multi-echo, use the combined field; for single-echo, use raw wrapped phase
            let phase_data = if meta.n_echoes > 1 {
                load_volume(&field_path)?
            } else {
                load_volume(&output.phase_scaled_path(&qsm_run.key, 1))?
            };

            let phase_f32: Vec<f32> = phase_data.iter().map(|&v| v as f32).collect();
            drop(phase_data);

            let params = qsm_core::inversion::TgvParams {
                alpha0: config.tgv_alphas[1] as f32,
                alpha1: config.tgv_alphas[0] as f32,
                iterations: config.tgv_iterations,
                erosions: config.tgv_erosions,
                step_size: config.tgv_step_size as f32,
                tol: config.tgv_tol as f32,
                fieldstrength: meta.field_strength as f32,
                te: meta.echo_times[0] as f32,
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
    } else if config.qsm_algorithm == QsmAlgorithm::Qsmart {
        // QSMART: two-stage SDF + iLSQR pipeline
        let chi_raw_path = output.chi_raw_path(&qsm_run.key);
        if !state.is_step_cached("qsmart") {
            let t = Instant::now();
            log::info!(
                "QSMART (iLSQR tol={:.0e}, max_iter={}, vasc_radius={}, sdf_radius={})",
                config.qsmart_ilsqr_tol, config.qsmart_ilsqr_max_iter,
                config.qsmart_vasc_sphere_radius, config.qsmart_sdf_spatial_radius,
            );
            progress("QSMART reconstruction");
            let field_ppm = load_volume(&field_path)?;
            let mask = load_mask(&mask_path)?;
            let bdir = meta.b0_direction;
            let qsmart_defaults = qsm_core::utils::QsmartParams::default();

            // Step 1: Vasculature detection
            progress("QSMART: vasculature detection");
            let mag_path = output.mag_path(&qsm_run.key, 1);
            let magnitude = if mag_path.exists() {
                load_volume(&mag_path)?
            } else {
                vec![1.0f64; nx * ny * nz]
            };
            let vasc_params = qsm_core::utils::VasculatureParams {
                sphere_radius: config.qsmart_vasc_sphere_radius,
                frangi_scale_range: qsmart_defaults.frangi_scale_range,
                frangi_scale_ratio: qsmart_defaults.frangi_scale_ratio,
                frangi_c: qsmart_defaults.frangi_c,
            };
            let vasc_pb: std::cell::RefCell<Option<ProgressBar>> = std::cell::RefCell::new(None);
            let vasc_mask = qsm_core::utils::generate_vasculature_mask_with_progress(
                &magnitude, &mask, nx, ny, nz, &vasc_params,
                |current: usize, total: usize| {
                    let mut pb_ref = vasc_pb.borrow_mut();
                    if pb_ref.is_none() && total > 0 {
                        *pb_ref = Some(create_progress_bar("Vasculature", total as u64));
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
            drop(magnitude);

            // Convert mask to f64 for QSMART functions
            let mask_f64: Vec<f64> = mask.iter().map(|&m| m as f64).collect();

            // Step 2: Weighted masks
            let w1 = qsm_core::utils::compute_weighted_mask_stage1(&mask_f64, &vasc_mask);
            let w2 = qsm_core::utils::compute_weighted_mask_stage2(&mask_f64, &vasc_mask, &vasc_mask);

            // Step 3: SDF stage 1 (background removal)
            progress("QSMART: SDF stage 1");
            let sdf_params1 = qsm_core::bgremove::SdfParams {
                sigma1: qsmart_defaults.sdf_sigma1_stage1,
                sigma2: qsmart_defaults.sdf_sigma2_stage1,
                spatial_radius: config.qsmart_sdf_spatial_radius,
                lower_lim: qsmart_defaults.sdf_lower_lim,
                curv_constant: qsmart_defaults.sdf_curv_constant,
                use_curvature: true,
            };
            let sdf1_pb: std::cell::RefCell<Option<ProgressBar>> = std::cell::RefCell::new(None);
            let lfs1 = qsm_core::bgremove::sdf::sdf_with_progress(
                &field_ppm, &w1, &vasc_mask, nx, ny, nz, &sdf_params1,
                |current: usize, total: usize| {
                    let mut pb_ref = sdf1_pb.borrow_mut();
                    if pb_ref.is_none() && total > 0 {
                        *pb_ref = Some(create_progress_bar("SDF-1", total as u64));
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

            // Step 4: iLSQR stage 1
            progress("QSMART: iLSQR stage 1");
            let (prog, _) = iter_progress_bar("iLSQR-1");
            let chi1 = qsm_core::inversion::ilsqr_with_progress(
                &lfs1, &mask, nx, ny, nz, vsx, vsy, vsz,
                bdir, config.qsmart_ilsqr_tol, config.qsmart_ilsqr_max_iter, prog,
            );

            // Step 5: SDF stage 2 (different sigma parameters)
            progress("QSMART: SDF stage 2");
            let sdf_params2 = qsm_core::bgremove::SdfParams {
                sigma1: qsmart_defaults.sdf_sigma1_stage2,
                sigma2: qsmart_defaults.sdf_sigma2_stage2,
                spatial_radius: config.qsmart_sdf_spatial_radius,
                lower_lim: qsmart_defaults.sdf_lower_lim,
                curv_constant: qsmart_defaults.sdf_curv_constant,
                use_curvature: true,
            };
            let sdf2_pb: std::cell::RefCell<Option<ProgressBar>> = std::cell::RefCell::new(None);
            let lfs2 = qsm_core::bgremove::sdf::sdf_with_progress(
                &field_ppm, &w2, &vasc_mask, nx, ny, nz, &sdf_params2,
                |current: usize, total: usize| {
                    let mut pb_ref = sdf2_pb.borrow_mut();
                    if pb_ref.is_none() && total > 0 {
                        *pb_ref = Some(create_progress_bar("SDF-2", total as u64));
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

            // Step 6: iLSQR stage 2
            progress("QSMART: iLSQR stage 2");
            let (prog, _) = iter_progress_bar("iLSQR-2");
            let chi2 = qsm_core::inversion::ilsqr_with_progress(
                &lfs2, &mask, nx, ny, nz, vsx, vsy, vsz,
                bdir, config.qsmart_ilsqr_tol, config.qsmart_ilsqr_max_iter, prog,
            );

            // Step 7: Combine with offset adjustment
            progress("QSMART: combining stages");
            let ppm = qsmart_defaults.ppm;
            let chi = qsm_core::utils::adjust_offset(
                &vasc_mask, &lfs1, &chi1, &chi2,
                nx, ny, nz, vsx, vsy, vsz, bdir, ppm,
            );

            save_volume(&chi_raw_path, &chi, &meta)?;
            state.mark_completed("qsmart", vec![chi_raw_path.clone()]);
            state.save(&state_path)?;
            log_step_done("QSMART", t);
        } else {
            log::info!("Skipping qsmart (cached)");
        }
    } else {
        // Standard path: bgremove → invert

        // --- Background removal (skipped when MEDI+SMV handles it internally) ---
        let skip_bgremove = config.qsm_algorithm == QsmAlgorithm::Medi && config.medi_smv;
        let local_field_path = output.local_field_path(&qsm_run.key);
        let bg_mask_path = output.bg_mask_path(&qsm_run.key);
        if skip_bgremove {
            log::info!("Skipping background removal (MEDI SMV handles it internally)");
        }
        if !skip_bgremove && !state.is_step_cached("bgremove") {
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
                    let mut r = config.vsharp_max_radius_factor * min_vox;
                    while r >= config.vsharp_min_radius_factor * max_vox { radii.push(r); r -= config.vsharp_min_radius_factor * max_vox; }
                    log::info!("Background removal (V-SHARP, {} radii, threshold={})", radii.len(), config.vsharp_threshold);
                    let (prog, _) = iter_progress_bar("V-SHARP");
                    qsm_core::bgremove::vsharp_with_progress(
                        &field_ppm, &mask, nx, ny, nz, vsx, vsy, vsz,
                        &radii, config.vsharp_threshold, prog,
                    )
                }
                Some(BfAlgorithm::Pdf) => {
                    let max_iter = ((nx * ny * nz) as f64).sqrt().ceil() as usize;
                    log::info!("Background removal (PDF, tol={:.0e}, max_iter={}, B0=[{:.2},{:.2},{:.2}])",
                        config.pdf_tol, max_iter, bdir.0, bdir.1, bdir.2);
                    let (prog, _) = iter_progress_bar("PDF");
                    let lf = qsm_core::bgremove::pdf_with_progress(
                        &field_ppm, &mask, nx, ny, nz, vsx, vsy, vsz,
                        bdir, config.pdf_tol, max_iter, prog,
                    );
                    (lf, mask.clone())
                }
                Some(BfAlgorithm::Lbv) => {
                    let max_iter = (3 * nx.max(ny).max(nz)).max(500);
                    log::info!("Background removal (LBV, tol={:.0e}, max_iter={})", config.lbv_tol, max_iter);
                    let (prog, _) = iter_progress_bar("LBV");
                    qsm_core::bgremove::lbv_with_progress(
                        &field_ppm, &mask, nx, ny, nz, vsx, vsy, vsz,
                        config.lbv_tol, max_iter, prog,
                    )
                }
                Some(BfAlgorithm::Ismv) => {
                    let radius = config.ismv_radius_factor * vsx.max(vsy).max(vsz);
                    log::info!("Background removal (iSMV, radius={:.1}, tol={:.0e}, max_iter={})",
                        radius, config.ismv_tol, config.ismv_max_iter);
                    let (prog, _) = iter_progress_bar("iSMV");
                    qsm_core::bgremove::ismv_with_progress(
                        &field_ppm, &mask, nx, ny, nz, vsx, vsy, vsz,
                        radius, config.ismv_tol, config.ismv_max_iter, prog,
                    )
                }
                Some(BfAlgorithm::Sharp) => {
                    let radius = config.sharp_radius_factor * vsx.min(vsy).min(vsz);
                    log::info!("Background removal (SHARP, radius={:.1}, threshold={})",
                        radius, config.sharp_threshold);
                    qsm_core::bgremove::sharp(
                        &field_ppm, &mask, nx, ny, nz, vsx, vsy, vsz,
                        radius, config.sharp_threshold,
                    )
                }
                None => {
                    return Err(QsmxtError::Config("bf_algorithm must be set for standard pipeline".to_string()));
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
            // When MEDI+SMV, use the total field (field_ppm) instead of local field
            let local_field = if skip_bgremove {
                load_volume(&field_path)?
            } else {
                load_volume(&local_field_path)?
            };
            let eroded_mask = if skip_bgremove {
                load_mask(&mask_path)?
            } else {
                load_mask(&bg_mask_path)?
            };
            let bdir = meta.b0_direction;

            let chi = match config.qsm_algorithm {
                QsmAlgorithm::Rts => {
                    log::info!("Dipole inversion (RTS, delta={}, mu={:.0e}, tol={:.0e}, max_iter={})",
                        config.rts_delta, config.rts_mu, config.rts_tol, config.rts_max_iter);
                    let (prog, _) = iter_progress_bar("RTS");
                    qsm_core::inversion::rts_with_progress(
                        &local_field, &eroded_mask, nx, ny, nz, vsx, vsy, vsz,
                        bdir, config.rts_delta, config.rts_mu, config.rts_rho,
                        config.rts_tol, config.rts_max_iter, config.rts_lsmr_iter,
                        prog,
                    )
                }
                QsmAlgorithm::Tv => {
                    log::info!("Dipole inversion (TV-ADMM, lambda={:.0e}, max_iter={})",
                        config.tv_lambda, config.tv_max_iter);
                    let (prog, _) = iter_progress_bar("TV");
                    qsm_core::inversion::tv_admm_with_progress(
                        &local_field, &eroded_mask, nx, ny, nz, vsx, vsy, vsz,
                        bdir, config.tv_lambda, config.tv_rho, config.tv_tol,
                        config.tv_max_iter, prog,
                    )
                }
                QsmAlgorithm::Tkd => {
                    log::info!("Dipole inversion (TKD, threshold={})", config.tkd_threshold);
                    qsm_core::inversion::tkd(
                        &local_field, &eroded_mask, nx, ny, nz, vsx, vsy, vsz,
                        bdir, config.tkd_threshold,
                    )
                }
                QsmAlgorithm::Tsvd => {
                    log::info!("Dipole inversion (TSVD, threshold={})", config.tsvd_threshold);
                    qsm_core::inversion::tsvd(
                        &local_field, &eroded_mask, nx, ny, nz, vsx, vsy, vsz,
                        bdir, config.tsvd_threshold,
                    )
                }
                QsmAlgorithm::Ilsqr => {
                    log::info!("Dipole inversion (iLSQR, tol={:.0e}, max_iter={})",
                        config.ilsqr_tol, config.ilsqr_max_iter);
                    let (prog, _) = iter_progress_bar("iLSQR");
                    qsm_core::inversion::ilsqr_with_progress(
                        &local_field, &eroded_mask, nx, ny, nz, vsx, vsy, vsz,
                        bdir, config.ilsqr_tol, config.ilsqr_max_iter, prog,
                    )
                }
                QsmAlgorithm::Tikhonov => {
                    log::info!("Dipole inversion (Tikhonov, lambda={:.0e})", config.tikhonov_lambda);
                    qsm_core::inversion::tikhonov(
                        &local_field, &eroded_mask, nx, ny, nz, vsx, vsy, vsz,
                        bdir, config.tikhonov_lambda, qsm_core::inversion::Regularization::Identity,
                    )
                }
                QsmAlgorithm::Nltv => {
                    log::info!("Dipole inversion (NLTV, lambda={:.0e}, max_iter={})",
                        config.nltv_lambda, config.nltv_max_iter);
                    let (prog, _) = iter_progress_bar("NLTV");
                    qsm_core::inversion::nltv_with_progress(
                        &local_field, &eroded_mask, nx, ny, nz, vsx, vsy, vsz,
                        bdir, config.nltv_lambda, config.nltv_mu, config.nltv_tol,
                        config.nltv_max_iter, config.nltv_newton_iter, prog,
                    )
                }
                QsmAlgorithm::Medi => {
                    log::info!("Dipole inversion (MEDI, lambda={:.0e}, max_iter={})",
                        config.medi_lambda, config.medi_max_iter);
                    let mag_path = output.mag_path(&qsm_run.key, 1);
                    let magnitude = if mag_path.exists() {
                        load_volume(&mag_path)?
                    } else {
                        vec![1.0f64; nx * ny * nz]
                    };
                    let n_std = vec![1.0f64; nx * ny * nz];
                    let (prog, _) = iter_progress_bar("MEDI");
                    qsm_core::inversion::medi_l1_with_progress(
                        &local_field, &n_std, &magnitude, &eroded_mask,
                        nx, ny, nz, vsx, vsy, vsz,
                        config.medi_lambda, bdir, false, config.medi_smv, config.medi_smv_radius,
                        1, config.medi_percentage, config.medi_cg_tol, config.medi_cg_max_iter,
                        config.medi_max_iter, config.medi_tol, prog,
                    )
                }
                QsmAlgorithm::Tgv => unreachable!("TGV handled separately"),
                QsmAlgorithm::Qsmart => unreachable!("QSMART handled separately"),
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

    } // end if config.do_qsm

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

/// Build mask from multiple sections, OR'd together.
#[allow(clippy::too_many_arguments)]
fn build_mask_from_sections(
    sections: &[crate::pipeline::config::MaskSection],
    phases: &[NiftiData],
    magnitudes: &[NiftiData],
    nx: usize, ny: usize, nz: usize,
    vsx: f64, vsy: f64, vsz: f64,
    echo_times: &[f64],
) -> crate::Result<Vec<u8>> {
    let n_voxels = nx * ny * nz;

    if sections.is_empty() {
        return Err(QsmxtError::Config("No mask sections configured".to_string()));
    }

    if sections.len() == 1 {
        // Single section: run directly
        return build_mask_from_section(&sections[0], phases, magnitudes, nx, ny, nz, vsx, vsy, vsz, echo_times);
    }

    // Multiple sections: run each, OR together
    log::info!("Building mask from {} sections (OR'd)", sections.len());
    let mut final_mask = vec![0u8; n_voxels];
    for (i, section) in sections.iter().enumerate() {
        log::info!("Mask section {} (input: {})", i + 1, section.input);
        let section_mask = build_mask_from_section(section, phases, magnitudes, nx, ny, nz, vsx, vsy, vsz, echo_times)?;
        for j in 0..n_voxels {
            final_mask[j] |= section_mask[j];
        }
    }
    let count: usize = final_mask.iter().map(|&m| m as usize).sum();
    log::info!("Combined mask: {}/{} voxels ({:.1}%)", count, n_voxels, 100.0 * count as f64 / n_voxels as f64);
    Ok(final_mask)
}

/// Build mask from a single section (input + ordered ops).
#[allow(clippy::too_many_arguments)]
fn build_mask_from_section(
    section: &crate::pipeline::config::MaskSection,
    phases: &[NiftiData],
    magnitudes: &[NiftiData],
    nx: usize, ny: usize, nz: usize,
    vsx: f64, vsy: f64, vsz: f64,
    echo_times: &[f64],
) -> crate::Result<Vec<u8>> {
    use crate::pipeline::config::{MaskOp, MaskThresholdMethod};

    let n_voxels = nx * ny * nz;
    let mut mask = vec![1u8; n_voxels];
    let input_data = resolve_masking_input(&section.input, phases, magnitudes, nx, ny, nz, echo_times);

    let all_ops = section.all_ops();
    for op in &all_ops {
        match op {
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
