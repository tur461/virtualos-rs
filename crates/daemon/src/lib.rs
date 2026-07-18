use std::path::PathBuf;

use engine::ContainerManager;

pub mod client;
pub mod service;

pub struct MyVirtualOs {
    // In a real daemon, we'd hold the manager and store, but we can instantiate them per request
    // or keep state. We'll create them inside each method for simplicity (same base dir).
    base_dir: PathBuf,
}

impl MyVirtualOs {
    fn manager(&self) -> ContainerManager {
        ContainerManager::new(&self.base_dir)
    }
}

impl Default for MyVirtualOs {
    fn default() -> Self {
        Self {
            base_dir: "/var/lib/docklet".into(),
        }
    }
}

// Put the actual method implementations in a separate module for clarity.
