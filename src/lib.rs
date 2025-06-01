// SPDX-License-Identifer: MIT
// A Custom Job System for a Future Project of Mine
// Copyright (C) 2025 A. Villavicencio

use std::{
    ops::Deref, sync::{Arc, mpsc}, thread::{self, Thread}
};
use parking_lot::{RwLock};
use generational_arena::Arena;
use num_cpus;

pub mod graph;

struct ManagerInner {
    cores: u8,
    jobs: Arena<graph::Job>, 
    threads: Vec<(Thread, mpsc::Sender<()>)>,
    cancelled: bool,
}

#[derive(Clone)]
pub struct Manager {
    inner: Arc<RwLock<ManagerInner>>,
}

fn task_handler(man: Manager, comms: mpsc::Receiver<()>) {
    'a: while true {
        // the manager will send when a job is available
        // otherwise, if it is dropped, the manager is dead
        // and we should die.
        match comms.recv() {
            Err(_) => break 'a,
            _ => {},
        }

        // while true is a misnomer, should be while job avalible
        'b: while true {
            let m = (*manager.inner).upgradable_read();
            let jid: Option<graph::JobId> = None;

            // first, check if a job is available
            for (id, j) in m.jobs.iter() {
                if j.dependency_check() {
                    // if so, take it
                    jid = Some(id);
                    break;
                }
            }

            match jid {
                Some(jobid) => {
                    // now, we claim our cash prize
                    let wman = m.upgrade();
                    let job = wman[jobid].clone();
                                
                    wman[jobid].in_progress = true;
                    // so we dont screw everything up
                    drop(wman);
                    // do the damn thing
                    job.execute();

                    // and finish the job
                    wman = (*manager.inner).write();
                    wman.jobs.remove(jobid);
                },
                None => break 'b,
            }

            // so we dont have to go full like try_recv
            if (*manager.inner).read().cancelled {
                break 'a;
            }
        }
    }
}

impl Manager {
    pub fn new(threads: u8) {
        let mut ret = Manager { inner: Arc::new(RwLock::new({}))};
        let mut man = (*ret.inner).write();
        
        man.cores = if threads == 0 {
            num_cpus::get_physical() as u8
        } else {
            threads.max(num_cpus::get() * 10)
        };

        for i in 0..man.cores {
            let (tx, rx) = mpsc::channel::<()>();
            let taskfn= move || task_handler(ret.clone(), rx);

            man.threads.push((*thread::spawn(taskfn).thread(), tx));
        }
    }

    fn register(&mut self, job: &mut graph::Job) -> JobId {
        let mut man = (*self.inner).write();
        let j = man.jobs.insert(job.clone());

        man.jobs[j].id = j;
        man.jobs[j].manager = self.clone();
        for (t, tx) in man.threads.iter_mut() {
            match tx.send(()) {
                Ok(_) => {},

                // if a job system thread dies, spin up a new one
                Err(_) => {
                    let mut rx;
                    let taskfn = move || task_handler(self.clone(), rx);

                    (*tx, rx) = mpsc::channel::<()>();
                    *t = *thread::spawn(taskfn).thread();

                    let _ = tx.send(());
                },
            }
        }

        drop(man);

        *job = (*self.inner).read().jobs[j].clone();

        j
    }
}