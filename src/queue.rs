use std::{collections::VecDeque, path::PathBuf};

#[derive(Debug, Default)]
pub struct JobQueue {
    jobs: VecDeque<PathBuf>,
}

impl JobQueue {
    pub fn new() -> Self {
        Self {
            jobs: VecDeque::new(),
        }
    }

    pub fn push(&mut self, path: PathBuf) -> bool {
        if self.jobs.contains(&path) {
            return false;
        }

        self.jobs.push_back(path);
        true
    }

    pub fn pop(&mut self) -> Option<PathBuf> {
        self.jobs.pop_front()
    }

    pub fn len(&self) -> usize {
        self.jobs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.jobs.is_empty()
    }
}
