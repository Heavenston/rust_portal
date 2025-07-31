use utils::default;

use std::collections::HashMap;
use shader_preprocessor as sprep;

pub fn compile_shader(
    source: &'_ str,
    defines: HashMap<String, sprep::Value>
) -> Result<String, sprep::Error<'_ >> {
    Ok(sprep::preprocess(&default(), defines, source)?)
}
