// SPDX-License-Identifer: MIT
// A Custom Job System for a Future Project of Mine
// Copyright (C) 2025 A. Villavicencio

use std::{
    boxed::Box, collections::VecDeque, sync::Arc, vec::Vec
};
use parking_lot::RwLock;
pub use generational_arena::Index as JobId;

#[derive(Clone)]
pub struct Job {
    id: JobId,
    dependencies: Vec<JobId>,
    manager: super::Manager,
    task: Arc<dyn FnOnce() -> ()>,
    in_progress: bool,
}

impl Job {
    pub fn dependency_check(&self) -> bool {
        let manager = (*self.manager.inner).read().unwrap();
        
        for dep in self.dependencies.as_slice().iter() {
            match manager.jobs.get(*dep) {
                Some(_) => return false,
                None => {}
            }
        }

        true
    }

    fn execute(&self) {
        while self.dependency_check() == false {
            // spin.
            ()
        }

        // here's the thing...
        (*self.task)();
    }
}