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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn queue_starts_empty() {
        let queue = JobQueue::new();

        assert!(queue.is_empty());
        assert_eq!(queue.len(), 0);
    }

    #[test]
    fn queue_pushes_and_pops_in_order() {
        let mut queue = JobQueue::new();

        let first = PathBuf::from("/downloads/first.rar");
        let second = PathBuf::from("/downloads/second.rar");

        assert!(queue.push(first.clone()));
        assert!(queue.push(second.clone()));

        assert_eq!(queue.len(), 2);
        assert_eq!(queue.pop(), Some(first));
        assert_eq!(queue.pop(), Some(second));
        assert!(queue.is_empty());
    }

    #[test]
    fn queue_ignores_duplicates() {
        let mut queue = JobQueue::new();

        let job = PathBuf::from("/downloads/movie.rar");

        assert!(queue.push(job.clone()));
        assert!(!queue.push(job));

        assert_eq!(queue.len(), 1);
    }
}
