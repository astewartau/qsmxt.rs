use super::common::{load_nifti, save_mask};
use crate::cli::BetArgs;

pub fn execute(args: BetArgs) -> crate::Result<()> {
    let nifti = load_nifti(&args.input)?;
    let (nx, ny, nz) = nifti.dims;
    let (vsx, vsy, vsz) = nifti.voxel_size;

    println!("Running BET on {} ({}x{}x{})", args.input.display(), nx, ny, nz);

    let mask = qsm_core::bet::run_bet(
        &nifti.data, nx, ny, nz, vsx, vsy, vsz,
        args.fractional_intensity, args.smoothness,
        args.gradient_threshold, args.iterations, args.subdivisions,
    );

    save_mask(&args.output, &mask, &nifti)?;

    let brain_voxels: usize = mask.iter().map(|&m| m as usize).sum();
    println!(
        "Mask saved to {} ({} brain voxels, {:.1}%)",
        args.output.display(), brain_voxels, 100.0 * brain_voxels as f64 / mask.len() as f64
    );
    Ok(())
}
