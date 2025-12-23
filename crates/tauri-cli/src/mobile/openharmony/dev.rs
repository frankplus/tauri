use super::{
  ensure_init, get_app, MobileTarget,
};
use crate::{
  dev::Options as DevOptions,
  helpers::{
    app_paths::tauri_dir,
    config::{get as get_tauri_config},
  },
  interface::{AppInterface, Interface},
  ConfigValue, Result,
};
use clap::{ArgAction, Parser};
use cargo_mobile2::{
  opts::{NoiseLevel, Profile},
};
use std::env::set_current_dir;

#[derive(Debug, Clone, Parser)]
#[clap(
  about = "Run your app in development mode on OpenHarmony",
  long_about = "Run your app in development mode on OpenHarmony."
)]
pub struct Options {
  /// Builds with the debug flag
  #[clap(short, long)]
  pub debug: bool,
  /// List of cargo features to activate
  #[clap(short, long, action = ArgAction::Append, num_args(0..))]
  pub features: Option<Vec<String>>,
  /// JSON strings or paths to JSON, JSON5 or TOML files to merge with the default configuration file
  #[clap(short, long)]
  pub config: Vec<ConfigValue>,
  /// Skip prompting for values
  #[clap(long, env = "CI")]
  pub ci: bool,
  /// Command line arguments passed to the runner.
  #[clap(last(true))]
  pub args: Vec<String>,
}

pub fn command(options: Options, noise_level: NoiseLevel) -> Result<()> {
    // 1. Run build
    let build_options = super::build::Options {
        debug: options.debug,
        features: options.features,
        config: options.config,
        ci: options.ci,
        args: options.args,
    };
    
    let hap_path = super::build::command(build_options, noise_level)?;
    
    // 2. Deploy and Run
    println!("Deploying and running OpenHarmony project...");
    
    let grep_cmd = String::new(); // TODO: Support grep pattern if passed in options (not currently in Options struct)

    let hap_path_str = hap_path.to_string_lossy();
    
    let shell_cmd = format!(r#"
hdc install {} && \
hdc shell hilog -r && \
hdc shell aa start -a EntryAbility -b com.tauri.basicapp && \
pid=$(timeout 0.5 hdc track-jpid | awk '$2=="com.tauri.basicapp"{{print $1}}') && \
hdc shell hilog -P "$pid"{}"#, hap_path_str, grep_cmd);

    let status = std::process::Command::new("sh")
        .arg("-c")
        .arg(shell_cmd)
        .status()
        .expect("Failed to execute device command");

    if !status.success() {
        return Err(crate::Error::GenericError("Device command failed".into()));
    }
    
    Ok(())
}
