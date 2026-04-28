use log::info;
use super::common::{load_nifti, load_mask, save_nifti};
use crate::cli::SwiArgs;
use crate::pipeline::phase;

pub fn execute(args: SwiArgs) -> crate::Result<()> {
    let phase_nifti = load_nifti(&args.phase)?;
    let mag_nifti = load_nifti(&args.magnitude)?;
    let (mask, _) = load_mask(&args.mask)?;

    let (nx, ny, nz) = phase_nifti.dims;
    let (vsx, vsy, vsz) = phase_nifti.voxel_size;

    // Scale and unwrap phase (SWI expects unwrapped phase)
    let mut phase_data = phase_nifti.data.clone();
    phase::scale_phase_to_pi(&mut phase_data);
    let unwrapped = qsm_core::unwrap::laplacian_unwrap(
        &phase_data, &mask, nx, ny, nz, vsx, vsy, vsz,
    );

    info!("Computing SWI ({}x{}x{})", nx, ny, nz);

    let swi = qsm_core::swi::calculate_swi_default(
        &unwrapped, &mag_nifti.data, &mask, nx, ny, nz, vsx, vsy, vsz,
    );

    save_nifti(&args.output, &swi, &phase_nifti)?;
    info!("SWI saved to {}", args.output.display());

    if args.mip {
        let mip = qsm_core::swi::create_mip_default(&swi, nx, ny, nz);
        let mip_path = args.mip_output.unwrap_or_else(|| args.output.with_extension("mip.nii"));
        save_nifti(&mip_path, &mip, &phase_nifti)?;
        info!("MIP saved to {}", mip_path.display());
    }

    Ok(())
}
