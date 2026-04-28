mod bids;
mod cli;
mod commands;
mod error;
mod executor;
mod pipeline;
#[cfg(test)]
mod testutils;
mod tui;

pub use error::Result;

use clap::Parser;
use cli::{Cli, Command};

fn main() {
    let cli = Cli::parse();

    // Run and TUI init their own loggers (to write to log files).
    // All other commands use a simple stderr logger.
    if !matches!(cli.command, Command::Run(_) | Command::Tui) {
        env_logger::Builder::new()
            .filter_level(log::LevelFilter::Info)
            .format_timestamp(None)
            .init();
    }

    let result = match cli.command {
        Command::Run(args) => commands::run::execute(args),
        Command::Init(args) => commands::init::execute(args),
        Command::Validate(args) => commands::validate::execute(args),
        Command::Slurm(args) => commands::slurm::execute(args),
        Command::Bet(args) => commands::bet::execute(args),
        Command::Mask(args) => commands::mask::execute(args),
        Command::Unwrap(args) => commands::unwrap::execute(args),
        Command::Bgremove(args) => commands::bgremove::execute(args),
        Command::Invert(args) => commands::invert::execute(args),
        Command::Swi(args) => commands::swi::execute(args),
        Command::R2star(args) => commands::r2star::execute(args),
        Command::T2star(args) => commands::t2star::execute(args),
        Command::Homogeneity(args) => commands::homogeneity::execute(args),
        Command::Resample(args) => commands::resample::execute(args),
        Command::QualityMap(args) => commands::quality_map::execute(args),
        Command::Dilate(args) => commands::dilate::execute(args),
        Command::Close(args) => commands::close::execute(args),
        Command::FillHoles(args) => commands::fill_holes::execute(args),
        Command::SmoothMask(args) => commands::smooth_mask::execute(args),
        Command::Tui => tui::run_tui(),
    };

    if let Err(e) = result {
        log::error!("{}", e);
        std::process::exit(1);
    }
}
