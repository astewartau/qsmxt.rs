use log::{info, warn};
use super::common::{load_nifti, load_mask, save_nifti};
use crate::cli::InvertCommand;

pub fn execute(cmd: InvertCommand) -> crate::Result<()> {
    let (common, chi) = match cmd {
        InvertCommand::Rts(args) => {
            let c = args.common;
            let field_nifti = load_nifti(&c.input)?;
            let (mask, _) = load_mask(&c.mask)?;
            let (nx, ny, nz) = field_nifti.dims;
            let (vsx, vsy, vsz) = field_nifti.voxel_size;
            let bdir = (c.b0_direction[0], c.b0_direction[1], c.b0_direction[2]);
            info!("Dipole inversion (RTS, {}x{}x{})", nx, ny, nz);

            let d = qsm_core::inversion::RtsParams::default();
            let chi = qsm_core::inversion::rts(
                &field_nifti.data, &mask, nx, ny, nz, vsx, vsy, vsz, bdir,
                args.delta.unwrap_or(d.delta),
                args.mu.unwrap_or(d.mu),
                args.rho.unwrap_or(d.rho),
                args.tol.unwrap_or(d.tol),
                args.max_iter.unwrap_or(d.max_iter),
                args.lsmr_iter.unwrap_or(d.lsmr_iter),
            );
            (c, (chi, field_nifti))
        }
        InvertCommand::Tv(args) => {
            let c = args.common;
            let field_nifti = load_nifti(&c.input)?;
            let (mask, _) = load_mask(&c.mask)?;
            let (nx, ny, nz) = field_nifti.dims;
            let (vsx, vsy, vsz) = field_nifti.voxel_size;
            let bdir = (c.b0_direction[0], c.b0_direction[1], c.b0_direction[2]);
            info!("Dipole inversion (TV, {}x{}x{})", nx, ny, nz);

            let d = qsm_core::inversion::TvParams::default();
            let chi = qsm_core::inversion::tv_admm(
                &field_nifti.data, &mask, nx, ny, nz, vsx, vsy, vsz, bdir,
                args.lambda.unwrap_or(d.lambda),
                args.rho.unwrap_or(d.rho),
                args.tol.unwrap_or(d.tol),
                args.max_iter.unwrap_or(d.max_iter),
            );
            (c, (chi, field_nifti))
        }
        InvertCommand::Tkd(args) => {
            let c = args.common;
            let field_nifti = load_nifti(&c.input)?;
            let (mask, _) = load_mask(&c.mask)?;
            let (nx, ny, nz) = field_nifti.dims;
            let (vsx, vsy, vsz) = field_nifti.voxel_size;
            let bdir = (c.b0_direction[0], c.b0_direction[1], c.b0_direction[2]);
            info!("Dipole inversion (TKD, {}x{}x{})", nx, ny, nz);

            let d = qsm_core::inversion::TkdParams::default();
            let chi = qsm_core::inversion::tkd(
                &field_nifti.data, &mask, nx, ny, nz, vsx, vsy, vsz, bdir,
                args.threshold.unwrap_or(d.threshold),
            );
            (c, (chi, field_nifti))
        }
        InvertCommand::Tsvd(args) => {
            let c = args.common;
            let field_nifti = load_nifti(&c.input)?;
            let (mask, _) = load_mask(&c.mask)?;
            let (nx, ny, nz) = field_nifti.dims;
            let (vsx, vsy, vsz) = field_nifti.voxel_size;
            let bdir = (c.b0_direction[0], c.b0_direction[1], c.b0_direction[2]);
            info!("Dipole inversion (TSVD, {}x{}x{})", nx, ny, nz);

            let d = qsm_core::inversion::TkdParams::default();
            let chi = qsm_core::inversion::tsvd(
                &field_nifti.data, &mask, nx, ny, nz, vsx, vsy, vsz, bdir,
                args.threshold.unwrap_or(d.threshold),
            );
            (c, (chi, field_nifti))
        }
        InvertCommand::Ilsqr(args) => {
            let c = args.common;
            let field_nifti = load_nifti(&c.input)?;
            let (mask, _) = load_mask(&c.mask)?;
            let (nx, ny, nz) = field_nifti.dims;
            let (vsx, vsy, vsz) = field_nifti.voxel_size;
            let bdir = (c.b0_direction[0], c.b0_direction[1], c.b0_direction[2]);
            info!("Dipole inversion (iLSQR, {}x{}x{})", nx, ny, nz);

            let d = qsm_core::inversion::IlsqrParams::default();
            let chi = qsm_core::inversion::ilsqr_simple(
                &field_nifti.data, &mask, nx, ny, nz, vsx, vsy, vsz, bdir,
                args.tol.unwrap_or(d.tol),
                args.max_iter.unwrap_or(d.max_iter),
            );
            (c, (chi, field_nifti))
        }
        InvertCommand::Tikhonov(args) => {
            let c = args.common;
            let field_nifti = load_nifti(&c.input)?;
            let (mask, _) = load_mask(&c.mask)?;
            let (nx, ny, nz) = field_nifti.dims;
            let (vsx, vsy, vsz) = field_nifti.voxel_size;
            let bdir = (c.b0_direction[0], c.b0_direction[1], c.b0_direction[2]);
            info!("Dipole inversion (Tikhonov, {}x{}x{})", nx, ny, nz);

            let d = qsm_core::inversion::TikhonovParams::default();
            let chi = qsm_core::inversion::tikhonov(
                &field_nifti.data, &mask, nx, ny, nz, vsx, vsy, vsz, bdir,
                args.lambda.unwrap_or(d.lambda), d.reg,
            );
            (c, (chi, field_nifti))
        }
        InvertCommand::Nltv(args) => {
            let c = args.common;
            let field_nifti = load_nifti(&c.input)?;
            let (mask, _) = load_mask(&c.mask)?;
            let (nx, ny, nz) = field_nifti.dims;
            let (vsx, vsy, vsz) = field_nifti.voxel_size;
            let bdir = (c.b0_direction[0], c.b0_direction[1], c.b0_direction[2]);
            info!("Dipole inversion (NLTV, {}x{}x{})", nx, ny, nz);

            let d = qsm_core::inversion::NltvParams::default();
            let chi = qsm_core::inversion::nltv(
                &field_nifti.data, &mask, nx, ny, nz, vsx, vsy, vsz, bdir,
                args.lambda.unwrap_or(d.lambda),
                args.mu.unwrap_or(d.mu),
                args.tol.unwrap_or(d.tol),
                args.max_iter.unwrap_or(d.max_iter),
                args.newton_iter.unwrap_or(d.newton_iter),
            );
            (c, (chi, field_nifti))
        }
        InvertCommand::Medi(args) => {
            let c = args.common;
            let field_nifti = load_nifti(&c.input)?;
            let (mask, _) = load_mask(&c.mask)?;
            let (nx, ny, nz) = field_nifti.dims;
            let (vsx, vsy, vsz) = field_nifti.voxel_size;
            let bdir = (c.b0_direction[0], c.b0_direction[1], c.b0_direction[2]);
            info!("Dipole inversion (MEDI, {}x{}x{})", nx, ny, nz);

            let d = qsm_core::inversion::MediParams::default();
            let n_voxels = field_nifti.data.len();
            let (n_std, magnitude) = if let Some(ref mag_path) = args.magnitude {
                let mag_nifti = load_nifti(mag_path)?;
                (vec![1.0f64; n_voxels], mag_nifti.data)
            } else {
                warn!("No --magnitude provided for MEDI; using uniform magnitude (results may be suboptimal)");
                (vec![1.0f64; n_voxels], vec![1.0f64; n_voxels])
            };
            let chi = qsm_core::inversion::medi_l1(
                &field_nifti.data, &n_std, &magnitude, &mask,
                nx, ny, nz, vsx, vsy, vsz,
                args.lambda.unwrap_or(d.lambda), bdir,
                args.merit.unwrap_or(d.merit),
                args.smv || d.smv,
                args.smv_radius.unwrap_or(d.smv_radius),
                args.data_weighting.unwrap_or(d.data_weighting),
                args.percentage.unwrap_or(d.percentage),
                args.cg_tol.unwrap_or(d.cg_tol),
                args.cg_max_iter.unwrap_or(d.cg_max_iter),
                args.max_iter.unwrap_or(d.max_iter),
                args.tol.unwrap_or(d.tol),
            );
            (c, (chi, field_nifti))
        }
        InvertCommand::Tgv(args) => {
            let c = args.common;
            let field_nifti = load_nifti(&c.input)?;
            let (mask, _) = load_mask(&c.mask)?;
            let (nx, ny, nz) = field_nifti.dims;
            let (vsx, vsy, vsz) = field_nifti.voxel_size;
            let bdir = (c.b0_direction[0], c.b0_direction[1], c.b0_direction[2]);
            info!("Dipole inversion (TGV, {}x{}x{})", nx, ny, nz);

            let d = qsm_core::inversion::TgvParams::default();
            let phase_f32: Vec<f32> = field_nifti.data.iter().map(|&v| v as f32).collect();
            let params = qsm_core::inversion::TgvParams {
                iterations: args.iterations.unwrap_or(d.iterations),
                erosions: args.erosions.unwrap_or(d.erosions),
                alpha1: args.alpha1.unwrap_or(d.alpha1 as f64) as f32,
                alpha0: args.alpha0.unwrap_or(d.alpha0 as f64) as f32,
                step_size: args.step_size.unwrap_or(d.step_size as f64) as f32,
                tol: args.tol.unwrap_or(d.tol as f64) as f32,
                fieldstrength: args.field_strength as f32,
                te: args.echo_time as f32,
            };
            let b0_f32 = (bdir.0 as f32, bdir.1 as f32, bdir.2 as f32);
            let chi_f32 = qsm_core::inversion::tgv_qsm(
                &phase_f32, &mask, nx, ny, nz,
                vsx as f32, vsy as f32, vsz as f32, &params, b0_f32,
            );
            let chi: Vec<f64> = chi_f32.iter().map(|&v| v as f64).collect();
            (c, (chi, field_nifti))
        }
    };

    let (chi_data, field_nifti) = chi;
    save_nifti(&common.output, &chi_data, &field_nifti)?;
    info!("Susceptibility map saved to {}", common.output.display());
    Ok(())
}
