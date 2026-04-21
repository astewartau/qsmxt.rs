use super::common::{load_nifti, load_mask, save_nifti};
use crate::cli::R2starArgs;
use crate::error::QsmxtError;

pub fn execute(args: R2starArgs) -> crate::Result<()> {
    if args.inputs.len() != args.echo_times.len() {
        return Err(QsmxtError::Config(format!(
            "Number of inputs ({}) must match number of echo times ({})",
            args.inputs.len(), args.echo_times.len()
        )));
    }

    let mut magnitudes = Vec::new();
    for path in &args.inputs {
        magnitudes.push(load_nifti(path)?);
    }

    let (nx, ny, nz) = magnitudes[0].dims;
    let n_voxels = nx * ny * nz;
    let n_echoes = magnitudes.len();
    let (mask, _) = load_mask(&args.mask)?;

    println!("Computing R2* from {} echoes ({}x{}x{})", n_echoes, nx, ny, nz);

    let mut interleaved = vec![0.0f64; n_voxels * n_echoes];
    for (echo_idx, mag) in magnitudes.iter().enumerate() {
        for vox in 0..n_voxels {
            interleaved[vox * n_echoes + echo_idx] = mag.data[vox];
        }
    }

    let (r2star_map, _s0_map) = qsm_core::utils::r2star_arlo(
        &interleaved, &mask, &args.echo_times, nx, ny, nz,
    );

    save_nifti(&args.output, &r2star_map, &magnitudes[0])?;
    println!("R2* map saved to {}", args.output.display());
    Ok(())
}
