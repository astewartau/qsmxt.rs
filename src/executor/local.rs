use log::{error, info};
use rayon::prelude::*;

use qsm_core::nifti_io;

use crate::bids::derivatives::DerivativeOutputs;
use crate::bids::discovery::QsmRun;
use crate::pipeline::config::PipelineConfig;
use crate::pipeline::runner;

/// Execute pipeline runs in parallel using Rayon.
pub fn execute_local(
    runs: Vec<QsmRun>,
    config: &PipelineConfig,
    output: &DerivativeOutputs,
    n_procs: usize,
) -> Vec<crate::Result<()>> {
    rayon::ThreadPoolBuilder::new()
        .num_threads(n_procs)
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
