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
    pub(crate) id: JobId,
    dependencies: Vec<JobId>,
    pub(crate) manager: super::Manager,
    task: Arc<dyn FnOnce() -> ()>,
    in_progress: bool,
}

impl Job {
    pub fn add_dependency(&mut self, job: JobId) {
        self.dependencies.push(job);
    }

    pub fn add_dependencies<T: Iterator<Item = JobId>>(&mut self, jobs: &T) {
        jobs.for_each(|j| self.dependencies.push(j));
    }

    pub fn new<F: FnOnce()>(f: F) -> Job {
        let mut me: Job;

        me.dependencies = Vec::new();
        me.task = Arc::new(f);
        me.in_progress = false;

        me
    }

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