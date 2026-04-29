use log::{info, debug};
use super::common::{load_nifti, load_mask, save_nifti};
use crate::cli::UnwrapCommand;
use crate::pipeline::phase;

pub fn execute(cmd: UnwrapCommand) -> crate::Result<()> {
    match cmd {
        UnwrapCommand::Laplacian(args) => {
            let phase_nifti = load_nifti(&args.common.input)?;
            let (mask, _) = load_mask(&args.common.mask)?;
            let (nx, ny, nz) = phase_nifti.dims;
            let (vsx, vsy, vsz) = phase_nifti.voxel_size;

            let mut phase_data = phase_nifti.data.clone();
            phase::scale_phase_to_pi(&mut phase_data);
            info!("Unwrapping phase (Laplacian, {}x{}x{})", nx, ny, nz);

            let unwrapped = qsm_core::unwrap::laplacian_unwrap(
                &phase_data, &mask, nx, ny, nz, vsx, vsy, vsz,
            );

            save_nifti(&args.common.output, &unwrapped, &phase_nifti)?;
            info!("Unwrapped phase saved to {}", args.common.output.display());
        }
        UnwrapCommand::Romeo(args) => {
            let phase_nifti = load_nifti(&args.common.input)?;
            let (mask, _) = load_mask(&args.common.mask)?;
            let (nx, ny, nz) = phase_nifti.dims;
            let mut phase_data = phase_nifti.data.clone();
            phase::scale_phase_to_pi(&mut phase_data);
            info!("Unwrapping phase (ROMEO, {}x{}x{})", nx, ny, nz);

            if args.no_phase_gradient_coherence || args.no_mag_coherence || args.no_mag_weight {
                debug!(
                    "ROMEO params: phase_gradient_coherence={}, mag_coherence={}, mag_weight={}",
                    !args.no_phase_gradient_coherence,
                    !args.no_mag_coherence,
                    !args.no_mag_weight,
                );
            }

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

            save_nifti(&args.common.output, &phase_mut, &phase_nifti)?;
            info!("Unwrapped phase saved to {}", args.common.output.display());
        }
    }
    Ok(())
}
