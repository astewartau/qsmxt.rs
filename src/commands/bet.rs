use qsm_core::nifti_io;

use crate::cli::BetArgs;
use crate::error::QsmxtError;

pub fn execute(args: BetArgs) -> crate::Result<()> {
    let nifti = nifti_io::read_nifti_file(&args.input)
        .map_err(|e| QsmxtError::NiftiIo(e))?;

    let (nx, ny, nz) = nifti.dims;
    let (vsx, vsy, vsz) = nifti.voxel_size;

    println!("Running BET on {} ({}x{}x{})", args.input.display(), nx, ny, nz);

    let mask = qsm_core::bet::run_bet(
        &nifti.data,
        nx, ny, nz,
        vsx, vsy, vsz,
        args.fractional_intensity,
        args.smoothness,
        args.gradient_threshold,
        args.iterations,
        args.subdivisions,
    );

    let mask_f64: Vec<f64> = mask.iter().map(|&m| m as f64).collect();
    nifti_io::save_nifti_to_file(
        &args.output,
        &mask_f64,
        nifti.dims,
        nifti.voxel_size,
        &nifti.affine,
    )
    .map_err(|e| QsmxtError::NiftiIo(e))?;

    let brain_voxels: usize = mask.iter().map(|&m| m as usize).sum();
    println!(
        "Mask saved to {} ({} brain voxels, {:.1}%)",
        args.output.display(),
        brain_voxels,
        100.0 * brain_voxels as f64 / mask.len() as f64
    );

    Ok(())
}
