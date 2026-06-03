//! Crate config

use crate::error::{Error, Result};
use grapple_utils::envs::{get, get_parse};
use std::sync::OnceLock;

static INSTANCE: OnceLock<Config> = OnceLock::new();

pub fn config() -> &'static Config {
    INSTANCE.get_or_init(|| {
        Config::load_from_env()
            .unwrap_or_else(|ex| panic!("FATAL - WHOLE LOADING CONF - Cause: {ex:?}"))
    })
}

#[allow(non_snake_case)]
#[derive(Debug)]
pub struct Config {}

impl Config {
    fn load_from_env() -> Result<Self> {
        Ok(Self {
            // load envs here using envs::
        })
    }

    pub fn init_from(cfg: Self) -> Result<()> {
        INSTANCE
            .set(cfg)
            .map_err(|_| Error::ConfigAlreadyInitialized)
    }
}
