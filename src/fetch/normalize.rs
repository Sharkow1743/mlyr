use std::sync::Arc;

use super::TrackQuery;

/// Optional query normalizer producing fallback query variants.
///
/// The pipeline always preserves the original query at `queries[0]` and
/// deduplicates any matching variants.
pub trait QueryNormalizer: Send + Sync {
    fn normalize(&self, query: &TrackQuery) -> Vec<TrackQuery>;
}

/// Default no-op normalizer that returns no extra variants.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoopNormalizer;

impl QueryNormalizer for NoopNormalizer {
    fn normalize(&self, _query: &TrackQuery) -> Vec<TrackQuery> {
        Vec::new()
    }
}

impl<N: QueryNormalizer + ?Sized> QueryNormalizer for &N {
    fn normalize(&self, query: &TrackQuery) -> Vec<TrackQuery> {
        (*self).normalize(query)
    }
}

impl<N: QueryNormalizer + ?Sized> QueryNormalizer for Box<N> {
    fn normalize(&self, query: &TrackQuery) -> Vec<TrackQuery> {
        (**self).normalize(query)
    }
}

impl<N: QueryNormalizer + ?Sized> QueryNormalizer for Arc<N> {
    fn normalize(&self, query: &TrackQuery) -> Vec<TrackQuery> {
        (**self).normalize(query)
    }
}

/// Builds an ordered, deduplicated query list where `queries[0]` is always `original`
pub fn build_query_list(
    original: &TrackQuery,
    normalizer: Option<&dyn QueryNormalizer>,
) -> Vec<TrackQuery> {
    let mut queries = vec![original.clone()];
    if let Some(norm) = normalizer {
        for variant in norm.normalize(original) {
            if !queries
                .iter()
                .any(|existing| queries_equivalent(existing, &variant))
            {
                queries.push(variant);
            }
        }
    }
    queries
}

fn queries_equivalent(a: &TrackQuery, b: &TrackQuery) -> bool {
    a.title == b.title
        && a.artist == b.artist
        && a.album == b.album
        && a.duration == b.duration
        && a.extensions.ptr_eq(&b.extensions)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestNormalizer;

    impl QueryNormalizer for TestNormalizer {
        fn normalize(&self, query: &TrackQuery) -> Vec<TrackQuery> {
            let mut same = query.clone();
            let mut clean = query.clone();
            clean.title = "Song".to_string();
            let dup = clean.clone();
            same.album = query.album.clone();
            vec![same, clean, dup]
        }
    }

    #[test]
    fn deduplicates_and_keeps_original_first() {
        let orig = TrackQuery::new("Song (feat. Guest)", "Artist");
        let list = build_query_list(&orig, Some(&TestNormalizer));
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].title, "Song (feat. Guest)");
        assert_eq!(list[1].title, "Song");
    }
}
