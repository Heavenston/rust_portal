#![feature(generic_const_exprs)]
#![feature(trait_alias)]

// for generic_const_exprs
#![expect(incomplete_features)]

#![forbid(unsafe_code)]

mod engine;
pub use engine::*;
pub mod builtin_shaders;
mod renderers;
