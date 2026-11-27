use std::sync::Arc;

use crate::model::Mlyr;

use super::{ScorerExtensions, TrackQuery};

/// Scores validated [`Mlyr`] candidates
///
/// Returning `<= 0.0` or `NaN` discards the candidate
pub trait Scorer: Send + Sync {
    fn score(
        &self,
        queries: &[TrackQuery],
        mlyr: &Mlyr,
        provider_id: Option<&str>,
        extensions: &ScorerExtensions,
    ) -> f32;
}

/// Default unopinionated scorer returning `1.0` for every candidate
#[derive(Debug, Clone, Copy, Default)]
pub struct DefaultScorer;

impl Scorer for DefaultScorer {
    fn score(
        &self,
        _queries: &[TrackQuery],
        _mlyr: &Mlyr,
        _provider_id: Option<&str>,
        _extensions: &ScorerExtensions,
    ) -> f32 {
        1.0
    }
}

impl<S: Scorer + ?Sized> Scorer for &S {
    fn score(
        &self,
        queries: &[TrackQuery],
        mlyr: &Mlyr,
        provider_id: Option<&str>,
        extensions: &ScorerExtensions,
    ) -> f32 {
        (*self).score(queries, mlyr, provider_id, extensions)
    }
}

impl<S: Scorer + ?Sized> Scorer for Box<S> {
    fn score(
        &self,
        queries: &[TrackQuery],
        mlyr: &Mlyr,
        provider_id: Option<&str>,
        extensions: &ScorerExtensions,
    ) -> f32 {
        (**self).score(queries, mlyr, provider_id, extensions)
    }
}

impl<S: Scorer + ?Sized> Scorer for Arc<S> {
    fn score(
        &self,
        queries: &[TrackQuery],
        mlyr: &Mlyr,
        provider_id: Option<&str>,
        extensions: &ScorerExtensions,
    ) -> f32 {
        (**self).score(queries, mlyr, provider_id, extensions)
    }
}
