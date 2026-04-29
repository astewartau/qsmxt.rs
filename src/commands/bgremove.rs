use log::info;
use super::common::{load_nifti, load_mask, save_nifti, save_mask};
use crate::cli::BgremoveCommand;

pub fn execute(cmd: BgremoveCommand) -> crate::Result<()> {
    let (common, local_field, eroded_mask) = match cmd {
        BgremoveCommand::Vsharp(args) => {
            let c = args.common;
            let field_nifti = load_nifti(&c.input)?;
            let (mask, _) = load_mask(&c.mask)?;
            let (nx, ny, nz) = field_nifti.dims;
            let (vsx, vsy, vsz) = field_nifti.voxel_size;
            info!("Background removal (V-SHARP, {}x{}x{})", nx, ny, nz);

            let d = qsm_core::bgremove::VsharpParams::default();
            let max_radius = args.max_radius_factor.unwrap_or(d.max_radius_factor) * vsx.min(vsy).min(vsz);
            let min_radius = args.min_radius_factor.unwrap_or(d.min_radius_factor) * vsx.max(vsy).max(vsz);
            let step = vsx.max(vsy).max(vsz);
            let mut radii = Vec::new();
            let mut r = max_radius;
            while r >= min_radius { radii.push(r); r -= step; }
            if radii.is_empty() { radii.push(min_radius); }
            let threshold = args.threshold.unwrap_or(d.threshold);

            let (lf, em) = qsm_core::bgremove::vsharp_with_progress(
                &field_nifti.data, &mask, nx, ny, nz, vsx, vsy, vsz,
                &radii, threshold, |_, _| {},
            );
            (c, (lf, field_nifti), em)
        }
        BgremoveCommand::Pdf(args) => {
            let c = args.common;
            let field_nifti = load_nifti(&c.input)?;
            let (mask, _) = load_mask(&c.mask)?;
            let (nx, ny, nz) = field_nifti.dims;
            let (vsx, vsy, vsz) = field_nifti.voxel_size;
            let bdir = (c.b0_direction[0], c.b0_direction[1], c.b0_direction[2]);
            info!("Background removal (PDF, {}x{}x{})", nx, ny, nz);

            let d = qsm_core::bgremove::PdfParams::default();
            let max_iter = ((nx * ny * nz) as f64).sqrt().ceil() as usize;
            let tol = args.tol.unwrap_or(d.tol);
            let lf = qsm_core::bgremove::pdf_with_progress(
                &field_nifti.data, &mask, nx, ny, nz, vsx, vsy, vsz,
                bdir, tol, max_iter, |_, _| {},
            );
            (c, (lf, field_nifti), mask)
        }
        BgremoveCommand::Lbv(args) => {
            let c = args.common;
            let field_nifti = load_nifti(&c.input)?;
            let (mask, _) = load_mask(&c.mask)?;
            let (nx, ny, nz) = field_nifti.dims;
            let (vsx, vsy, vsz) = field_nifti.voxel_size;
            info!("Background removal (LBV, {}x{}x{})", nx, ny, nz);

            let d = qsm_core::bgremove::LbvParams::default();
            let max_iter = ((nx * ny * nz) as f64).sqrt().ceil() as usize;
            let tol = args.tol.unwrap_or(d.tol);
            let (lf, em) = qsm_core::bgremove::lbv_with_progress(
                &field_nifti.data, &mask, nx, ny, nz, vsx, vsy, vsz,
                tol, max_iter, |_, _| {},
            );
            (c, (lf, field_nifti), em)
        }
        BgremoveCommand::Ismv(args) => {
            let c = args.common;
            let field_nifti = load_nifti(&c.input)?;
            let (mask, _) = load_mask(&c.mask)?;
            let (nx, ny, nz) = field_nifti.dims;
            let (vsx, vsy, vsz) = field_nifti.voxel_size;
            info!("Background removal (iSMV, {}x{}x{})", nx, ny, nz);

            let d = qsm_core::bgremove::IsmvParams::default();
            let radius = args.radius_factor.unwrap_or(d.radius_factor) * vsx.max(vsy).max(vsz);
            let tol = args.tol.unwrap_or(d.tol);
            let max_iter = args.max_iter.unwrap_or(d.max_iter);
            let (lf, em) = qsm_core::bgremove::ismv_with_progress(
                &field_nifti.data, &mask, nx, ny, nz, vsx, vsy, vsz,
                radius, tol, max_iter, |_, _| {},
            );
            (c, (lf, field_nifti), em)
        }
        BgremoveCommand::Sharp(args) => {
            let c = args.common;
            let field_nifti = load_nifti(&c.input)?;
            let (mask, _) = load_mask(&c.mask)?;
            let (nx, ny, nz) = field_nifti.dims;
            let (vsx, vsy, vsz) = field_nifti.voxel_size;
            info!("Background removal (SHARP, {}x{}x{})", nx, ny, nz);

            let d = qsm_core::bgremove::SharpParams::default();
            let radius = args.radius_factor.unwrap_or(d.radius_factor) * vsx.min(vsy).min(vsz);
            let threshold = args.threshold.unwrap_or(d.threshold);
            let (lf, em) = qsm_core::bgremove::sharp(
                &field_nifti.data, &mask, nx, ny, nz, vsx, vsy, vsz,
                radius, threshold,
            );
            (c, (lf, field_nifti), em)
        }
    };

    let (local_field_data, field_nifti) = local_field;
    save_nifti(&common.output, &local_field_data, &field_nifti)?;
    info!("Local field saved to {}", common.output.display());

    if let Some(ref mask_out) = common.output_mask {
        save_mask(mask_out, &eroded_mask, &field_nifti)?;
        info!("Eroded mask saved to {}", mask_out.display());
    }

    Ok(())
}
