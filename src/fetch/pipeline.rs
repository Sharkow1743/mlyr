use std::sync::Arc;
use std::time::Duration;

use futures::stream::{FuturesUnordered, Stream};

use crate::builders::MlyrBuilder;
use crate::model::Source;

use super::normalize::build_query_list;
use super::providers::DynLyricsProvider;
use super::{
    DefaultScorer, FetchContext, FetchError, QueryNormalizer, RawCandidate, ScoredCandidate,
    Scorer, ScorerExtensions, TrackQuery,
};

pub fn default_postprocess(provider_id: &str, builder: &mut MlyrBuilder, raw: &RawCandidate) {
    let provider_source = Source::new(
        "provider",
        Some(provider_id.to_string()),
        raw.source_url.clone(),
    );

    {
        let meta = builder.metadata_mut();

        if !meta.sources.contains(&provider_source) {
            meta.sources.push(provider_source);
        }

        for src in &raw.fetched_meta.sources {
            if src.validate().is_ok() && !meta.sources.contains(src) {
                meta.sources.push(src.clone());
            }
        }

        if meta.title.is_none() {
            meta.title = clean_non_empty(raw.fetched_meta.title.as_deref());
        }
        if meta.artist.is_none() {
            meta.artist = clean_non_empty(raw.fetched_meta.artist.as_deref());
        }
        if meta.album.is_none() {
            meta.album = clean_non_empty(raw.fetched_meta.album.as_deref());
        }
    }

    if builder.metadata_mut().length.is_none() {
        if let Some(track_len) = raw.fetched_meta.length {
            if track_len >= Duration::from_millis(1) {
                let can_clamp_all = builder.lines_mut().iter().all(|l| {
                    if l.end() <= track_len {
                        return true;
                    }
                    if l.offset >= track_len {
                        return false;
                    }
                    let new_dur = track_len - l.offset;
                    let min_dur = l
                        .segments
                        .last()
                        .map(|s| s.offset)
                        .unwrap_or(Duration::ZERO)
                        .max(Duration::from_millis(1));
                    new_dur >= min_dur
                });

                if can_clamp_all {
                    for line in builder.lines_mut().iter_mut() {
                        if line.end() > track_len {
                            line.duration = track_len - line.offset;
                        }
                    }
                    builder.metadata_mut().length = Some(track_len);
                }
            }
        }
    }
}

fn clean_non_empty(opt: Option<&str>) -> Option<String> {
    opt.map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

pub async fn run_provider<P: DynLyricsProvider + ?Sized>(
    provider: &P,
    query: &TrackQuery,
    ctx: &FetchContext,
    normalizer: Option<&dyn QueryNormalizer>,
    scorer: Option<&dyn Scorer>,
    scorer_extensions: &ScorerExtensions,
    pass_threshold: f32,
) -> Result<Vec<ScoredCandidate>, FetchError> {
    let queries = build_query_list(query, normalizer);
    run_provider_with_queries(
        provider,
        queries,
        ctx,
        scorer,
        scorer_extensions,
        pass_threshold,
    )
    .await
}

async fn run_provider_with_queries<P: DynLyricsProvider + ?Sized>(
    provider: &P,
    queries: Vec<TrackQuery>,
    ctx: &FetchContext,
    scorer: Option<&dyn Scorer>,
    scorer_extensions: &ScorerExtensions,
    pass_threshold: f32,
) -> Result<Vec<ScoredCandidate>, FetchError> {
    let default_scorer = DefaultScorer;
    let active_scorer: &dyn Scorer = scorer.unwrap_or(&default_scorer);
    let provider_id = provider.id().to_string();

    let mut results: Vec<ScoredCandidate> = Vec::new();
    let mut hit_threshold = false;
    let mut last_error: Option<FetchError> = None;

    for current_query in &queries {
        let raw_candidates = match provider.fetch_dyn(current_query, ctx).await {
            Ok(cands) => cands,
            Err(err) => {
                let is_rate_limited = matches!(err, FetchError::RateLimited { .. });
                last_error = Some(err);
                if is_rate_limited {
                    break;
                }
                continue;
            }
        };

        for mut raw in raw_candidates {
            provider.preprocess(&mut raw);

            let Ok(mlyr) = provider.parse(&raw) else {
                continue;
            };

            let mut builder = mlyr.into_builder();
            provider.postprocess(&mut builder, &raw);

            let Ok(mlyr) = builder.build() else {
                continue;
            };

            let score = active_scorer.score(
                &queries,
                &mlyr,
                Some(provider_id.as_str()),
                scorer_extensions,
            );

            if score.is_nan() || score <= 0.0 {
                continue;
            }

            if !pass_threshold.is_nan() && score >= pass_threshold {
                hit_threshold = true;
            }

            // Deduplicate matching lyrics lines and title/artist
            if let Some(existing) = results.iter_mut().find(|c| {
                c.mlyr.lyrics == mlyr.lyrics
                    && c.mlyr.metadata().and_then(|m| m.title.as_deref())
                        == mlyr.metadata().and_then(|m| m.title.as_deref())
                    && c.mlyr.metadata().and_then(|m| m.artist.as_deref())
                        == mlyr.metadata().and_then(|m| m.artist.as_deref())
            }) {
                if score > existing.score {
                    existing.score = score;
                    if existing.source_url.is_none() {
                        existing.source_url = raw.source_url;
                    }
                }
            } else {
                results.push(ScoredCandidate {
                    mlyr,
                    score,
                    provider_id: provider_id.clone(),
                    source_url: raw.source_url,
                });
            }
        }

        if hit_threshold {
            break;
        }
    }

    if results.is_empty() {
        if let Some(err) = last_error {
            return Err(err);
        }
    }

    results.sort_by(|a, b| b.score.total_cmp(&a.score));
    Ok(results)
}

pub fn fetch_stream<'a>(
    providers: impl IntoIterator<Item = Arc<dyn DynLyricsProvider>>,
    query: &'a TrackQuery,
    ctx: &'a FetchContext,
    normalizer: Option<&'a dyn QueryNormalizer>,
    scorer: Option<&'a dyn Scorer>,
    scorer_extensions: &'a ScorerExtensions,
    pass_threshold: f32,
) -> impl Stream<Item = (String, Result<Vec<ScoredCandidate>, FetchError>)> + Send + 'a {
    let queries = build_query_list(query, normalizer);
    let futures = FuturesUnordered::new();

    for provider in providers {
        let queries = queries.clone();
        futures.push(async move {
            let id = provider.id().to_string();
            let res = run_provider_with_queries(
                provider.as_ref(),
                queries,
                ctx,
                scorer,
                scorer_extensions,
                pass_threshold,
            )
            .await;
            (id, res)
        });
    }

    futures
}
