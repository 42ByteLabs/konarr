//! # Database Cache

use geekorm::prelude::*;
use std::sync::{Arc, RwLock};

use crate::KonarrError;

/// Database Cache
#[derive(Debug, Clone)]
pub struct DbCache<T: Clone> {
    cache: Arc<RwLock<Vec<T>>>,
}

impl<T: Clone> DbCache<T> {
    /// Create a new cache
    pub fn new() -> Self {
        Self::default()
    }

    /// Read the cache
    pub fn read(&self) -> Result<Vec<T>, KonarrError> {
        if let Ok(value) = self.cache.read() {
            Ok(value.clone())
        } else {
            Err(KonarrError::LockError("Cache Read".to_string()))
        }
    }

    /// Read the cache based on a page (offset, limit)
    pub fn read_page(&self, page: &Page) -> Result<Vec<T>, KonarrError> {
        if let Ok(value) = self.cache.read() {
            if let Some(slice) = value.get(page.offset() as usize..page.limit() as usize) {
                Ok(slice.to_vec())
            } else {
                Ok(vec![])
            }
        } else {
            Err(KonarrError::LockError("Cache Read".to_string()))
        }
    }

    /// Write the cache
    pub fn write(&self, value: Vec<T>) -> Result<(), KonarrError> {
        *self.cache.write().unwrap() = value;
        Ok(())
    }
}

impl<T: Clone> From<Vec<T>> for DbCache<T> {
    fn from(value: Vec<T>) -> Self {
        Self {
            cache: Arc::new(RwLock::new(value)),
        }
    }
}

impl<T: Clone> Default for DbCache<T> {
    fn default() -> Self {
        Self {
            cache: Arc::new(RwLock::new(Vec::new())),
        }
    }
}
