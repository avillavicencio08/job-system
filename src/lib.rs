// SPDX-License-Identifer: MIT
// A Custom Job System for a Future Project of Mine
// Copyright (C) 2025 A. Villavicencio

use std::{
    mem::MaybeUninit,
    ops::Deref,
    sync::{mpsc, Arc},
    thread::{self, JoinHandle, Thread}
};
use parking_lot::RwLock;
use generational_arena::Arena;
use num_cpus;

pub mod graph;
use graph::JobId;

struct ManagerInner {
    cores: u8,
    jobs: Arena<graph::Job>, 
    threads: Vec<(JoinHandle<()>, mpsc::Sender<()>)>,
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
            let jid: Option<JobId> = None;

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

            man.threads.push((thread::spawn(taskfn), tx));
        }
    }

    pub fn register(&mut self, job: &mut graph::Job) -> Result<JobId, ()> {
        let mut man = (*self.inner).write();
        let j;

        if man.cancelled {
            return Err(());
        }

        j = man.jobs.insert(job.clone());

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
                    *t = thread::spawn(taskfn);

                    let _ = tx.send(());
                },
            }
        }

        drop(man);

        *job = (*self.inner).read().jobs[j].clone();

        Ok(j)
    }

    pub fn job_status(&self, job: JobId) -> Option<bool> {
        let man = (*self.inner).read();

        match man.jobs.get(job) {
            Some(j) => Some(j.in_progress),
            None => None,
        }
    }

    pub fn active(&self) -> bool {
        let man = (*self.inner).read();

        man.jobs.iter().next().is_some()
    }

    pub fn num_in_progress(&self) -> usize {
        let man = (*self.inner).read();

        // in theory a for_each would be quicker but idk so...
        man.jobs.iter().filter(|(_, job)| job.in_progress).count()
    }

    pub fn num_jobs(&self) -> usize {
        let man = (*self.inner).read();

        if man.cancelled {
            self.num_in_progress()
        } else {
            man.jobs.iter().count()
        }
    }

    pub fn cancel_job(&mut self, job: JobId) -> Result<(), bool> {
        // prevent writing.
        let mut man = (*self.inner).upgradable_read();

        if man.cancelled {
            return Err(false);
        }

        match self.job_status(job) {
            Some(b) => {
                if b {
                    Err(true)
                } else {
                    man.with_upgraded(|inner| inner.jobs.remove(job));
                    Ok(())
                }
            },
            None => Err(false)
        }
    }

    pub fn cancel_all(&mut self, job: JobId) {
        let mut man = (*self.inner).upgradable_read();
        let indices: Vec<JobId>;

        if man.cancelled {
            return;
        }
        
        indices = man.jobs.iter().map(|i, _| i).collect();

        man.with_upgraded(|m|
            indices.into_iter().for_each(|i| {
                if m.jobs[i].in_progress == false {
                    m.jobs.remove(i);
                }
            })
        );
    }
    
    pub fn kill(&mut self) {
        let mut man = (*self.inner).write();
        let mut nthreads = Vec::new();

        man.cancelled = true;

        // keep the join handles, but get rid of tx.
        for (t, tx) in man.threads.into_iter() {
            drop(tx);
            nthreads.push((t, unsafe { MaybeUninit::uninit().assume_init() }));
        }

        man.threads = nthreads;
    }

    pub fn join(mut self) {
        let mut man = (*self.inner).write();

        if man.cancelled == false {
            drop(man);

            self.kill();

            man = (*self.inner).write();
        }

        for (t, _) in man.threads.into_iter() {
            _ = t.join();
        }
    }
}
