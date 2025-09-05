#![feature(iter_array_chunks)]

#![forbid(unsafe_code)]

mod graphics_kernel;
pub use graphics_kernel::*;
pub mod color;
mod compile_shader;
pub use compile_shader::*;
pub mod utils;
