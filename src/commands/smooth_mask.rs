use super::common::{load_nifti, save_nifti};
use crate::cli::SmoothMaskArgs;

pub fn execute(args: SmoothMaskArgs) -> crate::Result<()> {
    let nifti = load_nifti(&args.input)?;
    let (nx, ny, nz) = nifti.dims;
    let mask_f64: Vec<f64> = nifti.data.iter().map(|&v| if v > 0.0 { 1.0 } else { 0.0 }).collect();

    println!("Gaussian smoothing mask ({}x{}x{}, sigma={:.1}mm)", nx, ny, nz, args.sigma);

    let smoothed = qsm_core::utils::gaussian_smooth_3d(
        &mask_f64, [args.sigma, args.sigma, args.sigma], None, None, 3, nx, ny, nz,
    );
    let result: Vec<f64> = smoothed.iter().map(|&v| if v > 0.5 { 1.0 } else { 0.0 }).collect();

    save_nifti(&args.output, &result, &nifti)?;
    println!("Smoothed mask saved to {}", args.output.display());
    Ok(())
}
