#![feature(generic_const_exprs)]
#![feature(trait_alias)]

// for generic_const_exprs
#![expect(incomplete_features)]

pub mod utils;
mod engine;
pub use engine::*;
mod renderer;
pub use renderer::*;
mod uid;
pub use uid::*;
pub mod handle_map;
mod compile_shader;
pub mod color;
