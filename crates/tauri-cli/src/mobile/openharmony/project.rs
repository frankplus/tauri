use crate::Result;
use cargo_mobile2::util::cli::TextWrapper;
use handlebars::Handlebars;
use crate::helpers::template::JsonMap;

pub fn gen(
  config: &(),
  metadata: &(),
  (handlebars, map): (Handlebars, JsonMap),
  wrapper: &TextWrapper,
  skip_targets_install: bool,
) -> Result<()> {
    println!("OpenHarmony project scaffolding is not yet implemented.");
    println!("Please manually copy the 'openharmony' directory from an existing example.");
    Ok(())
}
