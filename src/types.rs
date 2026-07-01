use std::{
    path::PathBuf,
    time::Instant,
};

#[derive(Debug, Clone)]
pub struct ReleaseCandidate {
    pub path: PathBuf,
    pub last_seen: Instant,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ReleaseState {
    Active,
    Ready,
    Extracting,
    Finished,
    Failed,
}
