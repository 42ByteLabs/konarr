//! # Filter containers

/// Filter containers by name
///
pub fn filter_container(container: &str) -> bool {
    // Remove buildx containers
    if container.contains("buildx") {
        log::debug!("Skipping buildx container: {}", container);
        return true;
    }

    false
}
