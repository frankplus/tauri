use super::{
  ensure_init, get_app, init::command as init_command, log_finished, read_options, CliOptions,
  OptionsHandle, Target as MobileTarget,
};
use crate::{
  helpers::config::{Config as TauriConfig},
  ConfigValue, Result,
};
use clap::{Parser, Subcommand};
use cargo_mobile2::{
    config::app::App,
    opts::NoiseLevel,
};

mod build;
mod dev;
pub mod project;
// mod init; // We use the shared init from parent mod

#[derive(Parser)]
#[clap(
  author,
  version,
  about = "OpenHarmony commands",
  subcommand_required(true),
  arg_required_else_help(true)
)]
pub struct Cli {
  #[clap(subcommand)]
  command: Commands,
}

#[derive(Debug, Parser)]
#[clap(about = "Initialize OpenHarmony target in the project")]
pub struct InitOptions {
  /// Skip prompting for values
  #[clap(long, env = "CI")]
  ci: bool,
  /// Skips installing rust toolchains via rustup
  #[clap(long)]
  skip_targets_install: bool,
  /// JSON strings or paths to JSON, JSON5 or TOML files to merge with the default configuration file
  #[clap(short, long)]
  pub config: Vec<ConfigValue>,
}

#[derive(Subcommand)]
enum Commands {
  Init(InitOptions),
  Dev(dev::Options),
  Build(build::Options),
}

pub fn command(cli: Cli, verbosity: u8) -> Result<()> {
  let noise_level = NoiseLevel::from_occurrences(verbosity as u64);
  match cli.command {
    Commands::Init(options) => {
      crate::helpers::app_paths::resolve();
      init_command(
        MobileTarget::OpenHarmony,
        options.ci,
        false,
        options.skip_targets_install,
        options.config,
      )?
    }
    Commands::Dev(options) => dev::command(options, noise_level)?,
    Commands::Build(options) => build::command(options, noise_level).map(|_| ())?,
  }

  Ok(())
}

pub fn get_config(
  app: &App,
  config: &TauriConfig,
  features: Option<&Vec<String>>,
  cli_options: &CliOptions,
) -> ((), ()) {
  // TODO: Implement OpenHarmony specific config if needed
  ((), ())
}
