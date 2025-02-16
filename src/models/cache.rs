//! # Database Cache

use std::sync::{Arc, RwLock};

use crate::KonarrError;

/// Database Cache
#[derive(Debug, Clone)]
pub struct DbCache<T> {
    cache: Arc<RwLock<Vec<T>>>,
}

impl<T> DbCache<T> {
    /// Create a new cache
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new cache with a value
    pub fn from(value: Vec<T>) -> Self {
        Self {
            cache: Arc::new(RwLock::new(value)),
        }
    }

    /// Read the cache
    pub fn read(&self) -> Result<Arc<RwLock<Vec<T>>>, KonarrError> {
        Ok(self.cache.clone())
    }

    /// Write the cache
    pub fn write(&self, value: Vec<T>) -> Result<(), KonarrError> {
        *self.cache.write().unwrap() = value;
        Ok(())
    }
}

impl<T> Default for DbCache<T> {
    fn default() -> Self {
        Self {
            cache: Arc::new(RwLock::new(Vec::new())),
        }
    }
}
