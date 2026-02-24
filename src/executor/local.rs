use log::{error, info};
use rayon::prelude::*;

use qsm_core::nifti_io;

use crate::bids::derivatives::DerivativeOutputs;
use crate::bids::discovery::QsmRun;
use crate::pipeline::config::PipelineConfig;
use crate::pipeline::{memory, runner};

/// Configuration for local execution.
pub struct ExecutionConfig {
    /// Maximum number of threads (from --n-procs or auto-detected)
    pub n_procs: usize,
    /// Memory limit in bytes for concurrent run scheduling (None = no limit)
    pub mem_limit_bytes: Option<usize>,
}

/// Execute pipeline runs in parallel using Rayon.
///
/// When `mem_limit_bytes` is set, estimates per-run memory usage from
/// volume dimensions and limits concurrency to avoid exceeding the limit.
pub fn execute_local(
    runs: Vec<QsmRun>,
    config: &PipelineConfig,
    output: &DerivativeOutputs,
    exec_config: &ExecutionConfig,
) -> Vec<crate::Result<()>> {
    let n_threads = compute_concurrency(&runs, config, exec_config);

    rayon::ThreadPoolBuilder::new()
        .num_threads(n_threads)
        .build_global()
        .ok();

    runs.par_iter()
        .map(|run| {
            info!("Processing {}", run.key);

            let result = runner::run_qsm_pipeline(run, config, &|msg| {
                info!("{}: {}", run.key, msg);
            });

            match result {
                Ok(outputs) => {
                    // Create output directory
                    let qsm_path = output.qsm_path(&run.key);
                    if let Some(parent) = qsm_path.parent() {
                        std::fs::create_dir_all(parent)?;
                    }

                    // Save QSM
                    nifti_io::save_nifti_to_file(
                        &qsm_path,
                        &outputs.chi,
                        outputs.dims,
                        outputs.voxel_size,
                        &outputs.affine,
                    )
                    .map_err(|e| crate::error::QsmxtError::NiftiIo(e))?;
                    info!("{}: QSM saved to {}", run.key, qsm_path.display());

                    // Save mask
                    let mask_path = output.mask_path(&run.key);
                    let mask_f64: Vec<f64> =
                        outputs.mask.iter().map(|&m| m as f64).collect();
                    nifti_io::save_nifti_to_file(
                        &mask_path,
                        &mask_f64,
                        outputs.dims,
                        outputs.voxel_size,
                        &outputs.affine,
                    )
                    .map_err(|e| crate::error::QsmxtError::NiftiIo(e))?;

                    // Save SWI if computed
                    if let Some(ref swi) = outputs.swi {
                        let swi_path = output.swi_path(&run.key);
                        nifti_io::save_nifti_to_file(
                            &swi_path,
                            swi,
                            outputs.dims,
                            outputs.voxel_size,
                            &outputs.affine,
                        )
                        .map_err(|e| crate::error::QsmxtError::NiftiIo(e))?;
                    }

                    if let Some(ref mip) = outputs.swi_mip {
                        let mip_path = output.swi_mip_path(&run.key);
                        nifti_io::save_nifti_to_file(
                            &mip_path,
                            mip,
                            outputs.dims,
                            outputs.voxel_size,
                            &outputs.affine,
                        )
                        .map_err(|e| crate::error::QsmxtError::NiftiIo(e))?;
                    }

                    info!("{}: Done", run.key);
                    Ok(())
                }
                Err(e) => {
                    error!("{}: FAILED - {}", run.key, e);
                    Err(e)
                }
            }
        })
        .collect()
}

/// Compute the effective number of concurrent threads based on memory constraints.
fn compute_concurrency(
    runs: &[QsmRun],
    config: &PipelineConfig,
    exec_config: &ExecutionConfig,
) -> usize {
    let Some(mem_limit) = exec_config.mem_limit_bytes else {
        return exec_config.n_procs;
    };

    if runs.is_empty() {
        return exec_config.n_procs;
    }

    // Use the maximum estimate across all runs to handle heterogeneous dimensions
    let per_run = runs
        .iter()
        .map(|run| {
            let (nx, ny, nz) = run.dims;
            memory::estimate_peak_memory_bytes(
                nx,
                ny,
                nz,
                run.echoes.len(),
                run.has_magnitude,
                config,
            )
        })
        .max()
        .unwrap_or(0);

    let max_by_memory = if per_run > 0 {
        (mem_limit / per_run).max(1)
    } else {
        exec_config.n_procs
    };

    let effective = exec_config.n_procs.min(max_by_memory);

    info!(
        "Memory: {} per run (est.), {} available — {} concurrent run(s) (requested {})",
        memory::format_bytes(per_run),
        memory::format_bytes(mem_limit),
        effective,
        exec_config.n_procs,
    );

    effective
}
