use log::{error, info};

use crate::bids::discovery::{self, DiscoveryFilter};
use crate::bids::derivatives::DerivativeOutputs;
use crate::cli::RunArgs;
use crate::executor;
use crate::pipeline::config::PipelineConfig;
use crate::pipeline::memory;

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

    // Compute execution parameters
    let n_procs = args.n_procs.unwrap_or_else(|| {
        std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1)
    });

    let mem_limit_bytes = if args.no_mem_limit {
        None
    } else if let Some(gb) = args.mem_limit_gb {
        Some((gb * 1024.0 * 1024.0 * 1024.0) as usize)
    } else {
        // Auto-detect: use MemAvailable, reserve 1 GB for OS
        let available = memory::available_memory_bytes();
        let reserved = 1024 * 1024 * 1024; // 1 GB
        Some(available.saturating_sub(reserved))
    };

    if args.dry {
        println!("Pipeline: {}", config.description);
        println!("Algorithm: {:?}", config.qsm_algorithm);
        println!();
        for run in &runs {
            let (nx, ny, nz) = run.dims;
            let est = memory::estimate_peak_memory_bytes(
                nx, ny, nz, run.echoes.len(), run.has_magnitude, &config,
            );
            println!(
                "  {} ({} echo(es), {}x{}x{}, B0={:.1}T, est. {})",
                run.key,
                run.echoes.len(),
                nx, ny, nz,
                run.magnetic_field_strength,
                memory::format_bytes(est),
            );
        }
        if let Some(mem) = mem_limit_bytes {
            let per_run_max = runs
                .iter()
                .map(|r| {
                    memory::estimate_peak_memory_bytes(
                        r.dims.0, r.dims.1, r.dims.2,
                        r.echoes.len(), r.has_magnitude, &config,
                    )
                })
                .max()
                .unwrap_or(0);
            let max_concurrent = (mem.checked_div(per_run_max))
                .map(|v| v.max(1).min(n_procs))
                .unwrap_or(n_procs);
            println!();
            println!(
                "Memory: {} available, max {} concurrent run(s)",
                memory::format_bytes(mem),
                max_concurrent,
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
    let exec_config = executor::local::ExecutionConfig {
        n_procs,
        mem_limit_bytes,
        force: args.force,
        clean_intermediates: args.clean_intermediates,
    };

    let results = executor::local::execute_local(runs, &config, &output, &exec_config);

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
