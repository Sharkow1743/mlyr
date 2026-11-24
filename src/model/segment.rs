use std::time::Duration;

#[derive(Debug, PartialEq, Clone, Default)]
pub struct Segment {
    pub offset: Duration,
    pub text: String,
}
