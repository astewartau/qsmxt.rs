use super::common::run_mask_operation;
use crate::cli::DilateArgs;
use crate::pipeline::phase;

pub fn execute(args: DilateArgs) -> crate::Result<()> {
    let iters = args.iterations;
    run_mask_operation(&args.input, &args.output, "Dilating mask", |mask, nx, ny, nz| {
        phase::dilate_mask(mask, nx, ny, nz, iters)
    })
}
