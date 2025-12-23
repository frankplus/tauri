use crate::{
  error::{Context, ErrorExt},
  helpers::template,
  Result,
};
use cargo_mobile2::{
//   config::app::DEFAULT_ASSET_DIR,
  util::{
    self,
    cli::{TextWrapper},
    // prefix_path,
  },
};
use handlebars::Handlebars;
use include_dir::{include_dir, Dir};

use std::{
  fs,
  path::{Path, PathBuf},
};

const TEMPLATE_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/templates/mobile/openharmony");

pub fn gen(
  _config: &(),
  _metadata: &(),
  (handlebars, mut map): (Handlebars, template::JsonMap),
  _wrapper: &TextWrapper,
  _skip_targets_install: bool,
) -> Result<()> {
  let dest = std::env::current_dir().context("failed to get current dir")?
    .join("src-tauri")
    .join("gen")
    .join("openharmony");

  let (lib_name_str, identifier_str) = {
      let app = map.inner().get("app").and_then(|v| v.as_object()).context("app config missing")?;
      let lib_name = app.get("lib-name").and_then(|v| v.as_str()).unwrap_or("app");
      let identifier = app.get("identifier").and_then(|v| v.as_str()).unwrap_or("com.example.app");
      (lib_name.to_string(), identifier.to_string())
  };

  map.insert("lib-name", lib_name_str.clone());
  map.insert("reverse-domain-name", identifier_str);

  let mut created_dirs = Vec::new();
  template::render_with_generator(
    &handlebars,
    map.inner(),
    &TEMPLATE_DIR,
    &dest,
    &mut |path| generate_out_file(&path, &dest, &lib_name_str, &mut created_dirs),
  )
  .with_context(|| "failed to process template")?;

  Ok(())
}

fn generate_out_file(
  path: &Path,
  dest: &Path,
  lib_name: &str,
  created_dirs: &mut Vec<PathBuf>,
) -> std::io::Result<Option<fs::File>> {
  let mut path_buf = PathBuf::new();
  for component in path.components() {
    let component_str = component.as_os_str().to_string_lossy();
    if component_str == "lib_name_placeholder" {
      path_buf.push(lib_name);
    } else {
      path_buf.push(component.as_os_str());
    }
  }

  let final_path = dest.join(path_buf);

  let parent = final_path.parent().unwrap().to_path_buf();
  if !created_dirs.contains(&parent) {
    fs::create_dir_all(&parent)?;
    created_dirs.push(parent);
  }

  let mut options = fs::OpenOptions::new();
  options.write(true);

  #[cfg(unix)]
  if final_path.file_name().unwrap() == std::ffi::OsStr::new("hvigorw") {
    use std::os::unix::fs::OpenOptionsExt;
    options.mode(0o755);
  }

  if !final_path.exists() {
    options.create(true).open(final_path).map(Some)
  } else {
    Ok(None)
  }
}

