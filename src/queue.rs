use std::collections::VecDeque;
use std::path::PathBuf;

#[derive(Default)]
pub struct JobQueue {
    jobs: VecDeque<PathBuf>,
}

impl JobQueue {

    pub fn new() -> Self {
        Self {
            jobs: VecDeque::new(),
        }
    }

    pub fn push(&mut self, path: PathBuf) {

        if !self.jobs.contains(&path) {
            self.jobs.push_back(path);
        }
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
