// SPDX-License-Identifer: MIT
// A Custom Job System for a Future Project of Mine
// Copyright (C) 2025 A. Villavicencio

use std::{
    boxed::Box, collections::VecDeque, vec::Vec
};
use parking_lot::RwLock;
pub use generational_arena::Index as JobId;

#[derive(Clone)]
pub struct Job {
    id: JobId,
    dependencies: Vec<JobId>,
    manager: super::Manager,
    task: Box<dyn FnOnce() -> ()>,
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

    fn execute(&self) -> bool {
        let man;

        if self.dependency_check() == false {
            return false;
        }

        man = (*self.manager.inner).write();
        (*man).jobs[self.id].in_progress = true;
        drop(man);

        // here's the thing...
        self.task();

        man = (*self.manager.inner).write();
        (*man).jobs.remove(self.id);

        true
    }
}