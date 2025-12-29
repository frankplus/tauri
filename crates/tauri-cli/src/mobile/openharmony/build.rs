use super::{
  ensure_init, get_app, log_finished, MobileTarget, OptionsHandle,
};
use crate::{
  build::Options as BuildOptions,
  helpers::{
    app_paths::tauri_dir,
    config::{get as get_tauri_config},
    flock,
  },
  interface::{AppInterface, Interface, Options as InterfaceOptions},
  mobile::{write_options, CliOptions},
  ConfigValue, Result,
};
use clap::{ArgAction, Parser};
use cargo_mobile2::{
  opts::{NoiseLevel, Profile},
};
use std::env::set_current_dir;
use std::path::PathBuf;

#[derive(Debug, Clone, Parser)]
#[clap(
  about = "Build your app in release mode for OpenHarmony",
  long_about = "Build your app in release mode for OpenHarmony."
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

impl From<Options> for BuildOptions {
  fn from(options: Options) -> Self {
    Self {
      runner: None,
      debug: options.debug,
      target: None,
      features: options.features.unwrap_or_default(),
      bundles: None,
      no_bundle: false,
      config: options.config,
      args: options.args,
      ci: options.ci,
      skip_stapling: false,
      ignore_version_mismatches: false,
      no_sign: false,
    }
  }
}

pub fn command(options: Options, noise_level: NoiseLevel) -> Result<PathBuf> {
    crate::helpers::app_paths::resolve();

  let mut build_options: BuildOptions = options.clone().into();
  build_options.target = Some("aarch64-unknown-linux-ohos".into());

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

    let interface = AppInterface::new(tauri_config_, build_options.target.clone())?;
    
    let app = get_app(MobileTarget::OpenHarmony, tauri_config_, &interface);
    (interface, app)
  };
  
    let profile = if options.debug {
    Profile::Debug
  } else {
    Profile::Release
  };

  let tauri_path = tauri_dir();
  set_current_dir(tauri_path).expect("failed to set current directory to Tauri directory");

  ensure_init(
    &tauri_config,
    &app,
    app.root_dir().to_path_buf(),
    MobileTarget::OpenHarmony,
    options.ci,
  )?;

    // 1. Build Rust library
    // We can reuse 'interface.build' logic later, but for now lets stick to the manual command
    // to match the tools/cli implementation and ensure correct target.
    
    // We need to run this in src-tauri
    println!("Running cargo build...");
    let status = std::process::Command::new("cargo")
        .args(["build", "--target", "aarch64-unknown-linux-ohos", "--release"]) // Force release for now as per reference
        .status()
        .expect("Failed to execute cargo build");

    if !status.success() {
       return Err(crate::Error::GenericError("Cargo build failed".into()));
    }
    
    // 2. Prepare paths
    let root_dir = app.root_dir();
    let openharmony_dir = if root_dir.join("openharmony").exists() {
        root_dir.join("openharmony")
    } else if let Some(parent) = root_dir.parent() {
        if parent.join("openharmony").exists() {
             parent.join("openharmony")
        } else {
             // Default to keeping it as is, or error out?
             // Lets assume it might be generated inside gen like android/ios in future,
             // but for now allow sibling for back compat with basic-app
             root_dir.join("gen/openharmony")
        }
    } else {
        root_dir.join("openharmony")
    };
    
    if !openharmony_dir.exists() {
         return Err(crate::Error::GenericError(format!("OpenHarmony project directory not found at {}", openharmony_dir.display())));
    }

    let lib_dest_dir = openharmony_dir.join("entry/libs/arm64-v8a");
    std::fs::create_dir_all(&lib_dest_dir).expect("Failed to create libs directory");
    
    // 3. Copy Rust shared library
    let lib_name = app.lib_name();
    let file_name = format!("lib{}.so", lib_name);
    let mut lib_src = tauri_path.join("target/aarch64-unknown-linux-ohos/release").join(&file_name);
    
    if !lib_src.exists() {
        // Fallback for workspace usage (e.g. basic-app in repo)
        let workspace_path = tauri_path.join("../../../target/aarch64-unknown-linux-ohos/release").join(&file_name);
        if workspace_path.exists() {
             lib_src = workspace_path;
        }
    }
    
    let lib_dst = lib_dest_dir.join(&file_name);
    
    std::fs::copy(&lib_src, &lib_dst).map_err(|e| crate::Error::GenericError(format!("Failed to copy lib: {}", e)))?;
    println!("Copied {} to {}", lib_src.display(), lib_dest_dir.display());

     // 4. Copy libc++_shared.so
    let sdk_base = std::env::var("OHOS_BASE_SDK_HOME").expect("OHOS_BASE_SDK_HOME not set");
    let libcpp_src = PathBuf::from(format!("{}/18/native/llvm/lib/aarch64-linux-ohos/c++/libc++_shared.so", sdk_base));
    let libcpp_dst = lib_dest_dir.join("libc++_shared.so");
    
    if libcpp_src.exists() {
        std::fs::copy(&libcpp_src, &libcpp_dst).expect("Failed to copy libc++_shared.so");
        println!("Copied libc++_shared.so to {}", lib_dest_dir.display());
    } else {
         eprintln!("Warning: libc++_shared.so not found at {}. Skipping copy.", libcpp_src.display());
    }

    // 5. Run hvigorw assembleHap
    let hvigorw_cmd = {
        let env_path = std::env::var("OHOS_CMD_TOOLS")
            .ok()
            .map(|p| PathBuf::from(p).join("bin/hvigorw"))
            .filter(|p| p.exists())
            .map(|p| p.to_string_lossy().to_string());

        if let Some(path) = env_path {
            path
        } else if which::which("hvigorw").is_ok() {
            "hvigorw".to_string()
        } else {
            return Err(crate::Error::GenericError(
                "hvigorw not found. Please set OHOS_CMD_TOOLS or add hvigorw to your PATH.".into(),
            ));
        }
    };
    
    println!("Running {} assembleHap...", hvigorw_cmd);
    let hvigor_status = std::process::Command::new(hvigorw_cmd)
        .arg("assembleHap")
        .current_dir(&openharmony_dir)
        .status()
        .expect("Failed to execute hvigorw");
        
     if !hvigor_status.success() {
        return Err(crate::Error::GenericError("OpenHarmony build failed".into()));
     }

    Ok(openharmony_dir.join("entry/build/default/outputs/default/entry-default-signed.hap"))
}
