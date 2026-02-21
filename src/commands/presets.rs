use crate::cli::{Preset, PresetsArgs};
use crate::pipeline::config::{self, PipelineConfig};

pub fn execute(args: PresetsArgs) -> crate::Result<()> {
    match args.name {
        Some(name) => {
            let preset = match name.to_lowercase().as_str() {
                "gre" => Preset::Gre,
                "epi" => Preset::Epi,
                "bet" => Preset::Bet,
                "fast" => Preset::Fast,
                "body" => Preset::Body,
                _ => {
                    println!("Unknown preset: {}", name);
                    println!("Available presets:");
                    for (name, desc) in config::list_presets() {
                        println!("  {:8} {}", name, desc);
                    }
                    return Ok(());
                }
            };

            let cfg = PipelineConfig::from_preset(preset);
            println!("{}", cfg.to_annotated_toml());
        }
        None => {
            println!("Available pipeline presets:");
            println!();
            for (name, desc) in config::list_presets() {
                println!("  {:8} {}", name, desc);
            }
            println!();
            println!("Use 'qsmxt presets <name>' to see full configuration.");
            println!("Use 'qsmxt init --preset <name>' to generate a config file.");
        }
    }

    Ok(())
}
