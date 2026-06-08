// region:    --- Modules

use tracing_subscriber::EnvFilter;

// -- Modules
pub mod common;
pub mod compiler;
pub mod parser;
pub mod scanner;
pub mod table;
pub mod token;
pub mod vm;

mod error;
mod gc;
mod macros;
mod object;
mod precedence;

// -- Flatten
pub use error::{Error, Result};
pub use object::{Obj, ObjFunction, ObjString, ObjType};

// endregion: --- Modules

pub fn init() -> Result<()> {
    // LOGGING INITIALIZATION
    tracing_subscriber::fmt()
        .without_time() // For early development
        .with_target(false)
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    Ok(())
}
