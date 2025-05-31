// SPDX-License-Identifer: MIT
// A Custom Job System for a Future Project of Mine
// Copyright (C) 2025 A. Villavicencio

use std::{sync::Arc, thread::Thread};
use parking_lot::RwLock;
use generational_arena::Arena;

pub mod graph;

struct ManagerInner {
    cores: u8,
    jobs: Arena<graph::Job>,
    threads: Vec<(Thread, graph::JobId)>
}

#[derive(Clone)]
pub struct Manager {
    inner: Arc<RwLock<ManagerInner>>,
}