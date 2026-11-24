use std::time::Duration;

use super::Segment;

#[derive(Debug, PartialEq, Clone)]
pub struct ParallelLine {
    pub offset: Option<Duration>,
    pub voice: u8,
    pub segments: Vec<Segment>,
}
