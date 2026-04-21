use super::common::{load_nifti, save_nifti};
use crate::cli::HomogeneityArgs;

pub fn execute(args: HomogeneityArgs) -> crate::Result<()> {
    let nifti = load_nifti(&args.input)?;
    let (nx, ny, nz) = nifti.dims;
    let (vsx, vsy, vsz) = nifti.voxel_size;

    println!(
        "Applying inhomogeneity correction to {} ({}x{}x{}, sigma={:.1}mm)",
        args.input.display(), nx, ny, nz, args.sigma
    );

    let corrected = qsm_core::utils::makehomogeneous(
        &nifti.data, nx, ny, nz, vsx, vsy, vsz, args.sigma, args.nbox,
    );

    save_nifti(&args.output, &corrected, &nifti)?;
    println!("Corrected magnitude saved to {}", args.output.display());
    Ok(())
}
