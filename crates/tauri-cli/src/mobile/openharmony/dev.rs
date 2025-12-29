use super::{
  get_app, MobileTarget,
};
use crate::{
  helpers::{
    config::{get as get_tauri_config},
  },
  interface::{AppInterface, Interface},
  ConfigValue, Result,
};
use clap::{ArgAction, Parser};
use cargo_mobile2::{
  opts::{NoiseLevel},
};

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
    // 0. Resolve app info to get identifier
    // We need to resolve app paths before getting the config
    crate::helpers::app_paths::resolve();

    let tauri_config = get_tauri_config(
        tauri_utils::platform::Target::OpenHarmony,
        &options
            .config
            .iter()
            .map(|conf| &conf.0)
            .collect::<Vec<_>>(),
    )?;

    let (interface, app) = {
        let tauri_config_guard = tauri_config.lock().unwrap();
        let tauri_config_ = tauri_config_guard.as_ref().unwrap();

        let interface =
            AppInterface::new(tauri_config_, Some("aarch64-unknown-linux-ohos".into()))?;

        let app = get_app(MobileTarget::OpenHarmony, tauri_config_, &interface);
        (interface, app)
    };

    let bundle_identifier = app.identifier();

    // 1. Run build
    let build_options = super::build::Options {
        debug: options.debug,
        features: options.features,
        config: options.config,
        ci: options.ci,
        args: options.args,
    };

    let hap_path = super::build::run(build_options, &tauri_config, &interface, &app, noise_level)?;

    // 2. Deploy and Run
    println!("Deploying and running OpenHarmony project...");

    let grep_cmd = String::new(); // TODO: Support grep pattern if passed in options (not currently in Options struct)

    let hap_path_str = hap_path.to_string_lossy();

    let shell_cmd = format!(
        r#"
hdc install {} && \
hdc shell hilog -r && \
hdc shell aa start -a EntryAbility -b {} && \
pid=$(timeout 0.5 hdc track-jpid | awk '$2=="{}"{{print $1}}') && \
hdc shell hilog -P "$pid"{}"#,
        hap_path_str, bundle_identifier, bundle_identifier, grep_cmd
    );

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
