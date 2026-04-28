use log::info;
use super::common::{load_nifti, load_mask, save_nifti};
use crate::cli::{UnwrapAlgorithmArg, UnwrapArgs};
use crate::pipeline::phase;

pub fn execute(args: UnwrapArgs) -> crate::Result<()> {
    let phase_nifti = load_nifti(&args.input)?;
    let (mask, _) = load_mask(&args.mask)?;

    let (nx, ny, nz) = phase_nifti.dims;
    let (vsx, vsy, vsz) = phase_nifti.voxel_size;

    // Scale phase to [-pi, pi] range
    let mut phase_data = phase_nifti.data.clone();
    phase::scale_phase_to_pi(&mut phase_data);

    info!("Unwrapping phase ({:?}, {}x{}x{})", args.algorithm, nx, ny, nz);

    let unwrapped = match args.algorithm {
        UnwrapAlgorithmArg::Laplacian => {
            qsm_core::unwrap::laplacian_unwrap(&phase_data, &mask, nx, ny, nz, vsx, vsy, vsz)
        }
        UnwrapAlgorithmArg::Romeo => {
            let mag = if let Some(ref mag_path) = args.magnitude {
                load_nifti(mag_path)?.data
            } else {
                vec![1.0f64; phase_data.len()]
            };

            let weights = qsm_core::unwrap::calculate_weights_single_echo(
                &phase_data, &mag, &mask, nx, ny, nz,
            );

            let mut phase_mut = phase_data;
            let mut mask_mut = mask.clone();
            let (si, sj, sk) = phase::mask_center_of_mass(&mask, nx, ny, nz);

            qsm_core::region_grow::grow_region_unwrap(
                &mut phase_mut, &weights, &mut mask_mut, nx, ny, nz, si, sj, sk,
            );

            phase_mut
        }
    };

    save_nifti(&args.output, &unwrapped, &phase_nifti)?;
    info!("Unwrapped phase saved to {}", args.output.display());
    Ok(())
}
