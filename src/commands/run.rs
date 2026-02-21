use log::{error, info};

use crate::bids::discovery::{self, DiscoveryFilter};
use crate::bids::derivatives::DerivativeOutputs;
use crate::cli::RunArgs;
use crate::executor;
use crate::pipeline::config::PipelineConfig;

pub fn execute(args: RunArgs) -> crate::Result<()> {
    // Build config: preset -> file -> CLI overrides
    let mut config = if let Some(ref path) = args.config {
        PipelineConfig::from_file(path)?
    } else if let Some(preset) = args.preset {
        PipelineConfig::from_preset(preset)
    } else {
        PipelineConfig::default()
    };

    config.apply_run_overrides(&args);
    config.validate()?;

    // Discover BIDS runs
    let filter = DiscoveryFilter {
        subjects: args.subjects.clone(),
        sessions: args.sessions.clone(),
        acquisitions: args.acquisitions.clone(),
        runs: args.runs.clone(),
        num_echoes: args.num_echoes,
    };

    let runs = discovery::discover_runs(&args.bids_dir, &filter)?;

    if runs.is_empty() {
        error!("No QSM-compatible runs found in {}", args.bids_dir.display());
        return Ok(());
    }

    // Count unique subjects
    let mut subjects: Vec<&str> = runs.iter().map(|r| r.key.subject.as_str()).collect();
    subjects.sort();
    subjects.dedup();

    info!(
        "Discovered {} run(s) across {} subject(s)",
        runs.len(),
        subjects.len()
    );

    if args.dry {
        println!("Pipeline: {}", config.description);
        println!("Algorithm: {:?}", config.qsm_algorithm);
        println!();
        for run in &runs {
            println!(
                "  {} ({} echo(es), B0={:.1}T, TEs={:?}s)",
                run.key, run.echoes.len(), run.magnetic_field_strength, run.echo_times
            );
        }
        return Ok(());
    }

    // Create output directory
    let output = DerivativeOutputs::new(&args.output_dir);
    output.write_dataset_description()?;

    // Save config to output
    let config_path = args.output_dir.join("pipeline_config.toml");
    std::fs::write(&config_path, config.to_annotated_toml())?;

    // Execute
    let n_procs = args.n_procs.unwrap_or_else(|| {
        std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1)
    });

    let results = executor::local::execute_local(runs, &config, &output, n_procs);

    let failures: Vec<_> = results.iter().filter(|r| r.is_err()).collect();
    if !failures.is_empty() {
        error!("{} run(s) failed:", failures.len());
        for f in &failures {
            if let Err(e) = f {
                error!("  {}", e);
            }
        }
        return Err(crate::error::QsmxtError::Algorithm {
            stage: "pipeline".to_string(),
            message: format!("{} run(s) failed", failures.len()),
        });
    }

    info!("All runs completed successfully");
    Ok(())
}
