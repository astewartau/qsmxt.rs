use std::path::{Path, PathBuf};
use std::process::Command;

use log::info;

use crate::bids::discovery::QsmRun;
use crate::pipeline::config::PipelineConfig;

/// Generate SLURM job scripts for all runs.
pub fn generate_all_slurm(
    runs: &[QsmRun],
    bids_dir: &Path,
    output_dir: &Path,
    _config: &PipelineConfig,
    account: &str,
    partition: Option<&str>,
    time: &str,
    mem_gb: usize,
    cpus: usize,
) -> crate::Result<Vec<PathBuf>> {
    let slurm_dir = output_dir.join("slurm");
    std::fs::create_dir_all(&slurm_dir)?;

    // Save config for SLURM jobs to reference
    let config_path = output_dir.join("pipeline_config.toml");
    std::fs::write(&config_path, _config.to_annotated_toml())?;

    let qsmxt_bin = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("qsmxt"));

    let mut scripts = Vec::new();

    for run in runs {
        let job_name = format!("qsmxt_{}", run.key);
        let script_path = slurm_dir.join(format!("{}.sh", job_name));

        let partition_line = match partition {
            Some(p) => format!("#SBATCH --partition={}", p),
            None => String::new(),
        };

        let session_flag = match &run.key.session {
            Some(ses) => format!("--sessions {}", ses),
            None => String::new(),
        };

        let run_flag = match &run.key.run {
            Some(r) => format!("--runs {}", r),
            None => String::new(),
        };

        let acq_flag = match &run.key.acquisition {
            Some(a) => format!("--acquisitions {}", a),
            None => String::new(),
        };

        let script = format!(
            r#"#!/bin/bash
#SBATCH --job-name={job_name}
#SBATCH --account={account}
{partition_line}
#SBATCH --time={time}
#SBATCH --mem={mem}G
#SBATCH --cpus-per-task={cpus}
#SBATCH --output={job_name}_%j.out
#SBATCH --error={job_name}_%j.err

{binary} run {bids_dir} {output_dir} \
  --config {config} \
  --subjects {subject} \
  {session_flag} \
  {run_flag} \
  {acq_flag} \
  --n-procs {cpus}
"#,
            job_name = job_name,
            account = account,
            partition_line = partition_line,
            time = time,
            mem = mem_gb,
            cpus = cpus,
            binary = qsmxt_bin.display(),
            bids_dir = bids_dir.display(),
            output_dir = output_dir.display(),
            config = config_path.display(),
            subject = run.key.subject,
            session_flag = session_flag,
            run_flag = run_flag,
            acq_flag = acq_flag,
        );

        std::fs::write(&script_path, script)?;
        scripts.push(script_path);
    }

    Ok(scripts)
}

/// Submit SLURM scripts using sbatch.
pub fn submit_scripts(scripts: &[PathBuf]) -> crate::Result<()> {
    for script in scripts {
        info!("Submitting {}", script.display());
        let output = Command::new("sbatch")
            .arg(script)
            .output()
            .map_err(|e| crate::error::QsmxtError::Slurm(format!("sbatch failed: {}", e)))?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            info!("  {}", stdout.trim());
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(crate::error::QsmxtError::Slurm(format!(
                "sbatch failed for {}: {}",
                script.display(),
                stderr
            )));
        }
    }
    Ok(())
}
