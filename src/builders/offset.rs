use std::time::Duration;

pub trait IntoOffset {
    fn into_offset(self) -> Option<Duration>;
}

impl IntoOffset for Duration {
    fn into_offset(self) -> Option<Duration> {
        Some(self)
    }
}

impl IntoOffset for f64 {
    fn into_offset(self) -> Option<Duration> {
        if self.is_finite() && self >= 0.0 {
            let ms = (self * 1_000.0).round();
            if ms <= u64::MAX as f64 {
                return Some(Duration::from_millis(ms as u64));
            }
        }
        None
    }
}

impl IntoOffset for f32 {
    fn into_offset(self) -> Option<Duration> {
        (self as f64).into_offset()
    }
}

impl IntoOffset for u64 {
    fn into_offset(self) -> Option<Duration> {
        Some(Duration::from_secs(self))
    }
}

impl IntoOffset for u32 {
    fn into_offset(self) -> Option<Duration> {
        Some(Duration::from_secs(self as u64))
    }
}

impl<T: IntoOffset> IntoOffset for Option<T> {
    fn into_offset(self) -> Option<Duration> {
        self.and_then(|v| v.into_offset())
    }
}

impl IntoOffset for () {
    fn into_offset(self) -> Option<Duration> {
        None
    }
}
