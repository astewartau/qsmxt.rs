use super::common::{load_nifti, load_mask, save_nifti};
use crate::cli::{InvertArgs, QsmAlgorithmArg};
use crate::error::QsmxtError;

pub fn execute(args: InvertArgs) -> crate::Result<()> {
    let field_nifti = load_nifti(&args.input)?;
    let (mask, _) = load_mask(&args.mask)?;

    let (nx, ny, nz) = field_nifti.dims;
    let (vsx, vsy, vsz) = field_nifti.voxel_size;
    let bdir = (args.b0_direction[0], args.b0_direction[1], args.b0_direction[2]);

    println!("Dipole inversion ({:?}, {}x{}x{})", args.algorithm, nx, ny, nz);

    let chi: Vec<f64> = match args.algorithm {
        QsmAlgorithmArg::Rts => qsm_core::inversion::rts(
            &field_nifti.data, &mask, nx, ny, nz, vsx, vsy, vsz,
            bdir, args.rts_delta, args.rts_mu, args.rts_rho,
            args.rts_tol, args.rts_max_iter, args.rts_lsmr_iter,
        ),
        QsmAlgorithmArg::Tv => qsm_core::inversion::tv_admm(
            &field_nifti.data, &mask, nx, ny, nz, vsx, vsy, vsz,
            bdir, args.tv_lambda, args.tv_rho, args.tv_tol, args.tv_max_iter,
        ),
        QsmAlgorithmArg::Tkd => qsm_core::inversion::tkd(
            &field_nifti.data, &mask, nx, ny, nz, vsx, vsy, vsz,
            bdir, args.tkd_threshold,
        ),
        QsmAlgorithmArg::Tgv => {
            let fs = args.field_strength.ok_or_else(|| {
                QsmxtError::Config("--field-strength required for TGV".to_string())
            })?;
            let te = args.echo_time.ok_or_else(|| {
                QsmxtError::Config("--echo-time required for TGV".to_string())
            })?;
            let phase_f32: Vec<f32> = field_nifti.data.iter().map(|&v| v as f32).collect();
            let params = qsm_core::inversion::TgvParams {
                iterations: args.tgv_iterations, erosions: args.tgv_erosions,
                fieldstrength: fs as f32, te: te as f32, ..Default::default()
            };
            let b0_f32 = (bdir.0 as f32, bdir.1 as f32, bdir.2 as f32);
            let chi_f32 = qsm_core::inversion::tgv_qsm(
                &phase_f32, &mask, nx, ny, nz,
                vsx as f32, vsy as f32, vsz as f32, &params, b0_f32,
            );
            chi_f32.iter().map(|&v| v as f64).collect()
        }
    };

    save_nifti(&args.output, &chi, &field_nifti)?;
    println!("Susceptibility map saved to {}", args.output.display());
    Ok(())
}
