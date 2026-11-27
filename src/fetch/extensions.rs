use std::any::{Any, TypeId};
use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::sync::Arc;

/// A key for [`ProviderExtensions`].
pub struct Key<T: Any + Send + Sync> {
    name: &'static str,
    _marker: PhantomData<fn() -> T>,
}

impl<T: Any + Send + Sync> Key<T> {
    pub const fn new(name: &'static str) -> Self {
        Self {
            name,
            _marker: PhantomData,
        }
    }

    pub const fn name(&self) -> &'static str {
        self.name
    }
}

impl<T: Any + Send + Sync> Copy for Key<T> {}

impl<T: Any + Send + Sync> Clone for Key<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: Any + Send + Sync> fmt::Debug for Key<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Key").field("name", &self.name).finish()
    }
}

impl<T: Any + Send + Sync> PartialEq for Key<T> {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl<T: Any + Send + Sync> Eq for Key<T> {}

impl<T: Any + Send + Sync> Hash for Key<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}

/// Named type-erased extension map carried on [`TrackQuery`](super::TrackQuery)
/// so independent providers can share inputs via [`Key<T>`].
#[derive(Clone, Default)]
pub struct ProviderExtensions {
    map: HashMap<Cow<'static, str>, Arc<dyn Any + Send + Sync>>,
}

impl ProviderExtensions {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    pub fn insert<T: Any + Send + Sync>(&mut self, key: Key<T>, value: T) {
        self.map.insert(Cow::Borrowed(key.name), Arc::new(value));
    }

    pub fn with<T: Any + Send + Sync>(mut self, key: Key<T>, value: T) -> Self {
        self.insert(key, value);
        self
    }

    pub fn insert_named<T: Any + Send + Sync>(
        &mut self,
        name: impl Into<Cow<'static, str>>,
        value: T,
    ) {
        self.map.insert(name.into(), Arc::new(value));
    }

    pub fn get<T: Any + Send + Sync>(&self, key: Key<T>) -> Option<&T> {
        self.map.get(key.name).and_then(|v| v.downcast_ref::<T>())
    }

    pub fn get_arc<T: Any + Send + Sync>(&self, key: Key<T>) -> Option<Arc<T>> {
        self.map
            .get(key.name)
            .and_then(|v| Arc::clone(v).downcast::<T>().ok())
    }

    pub fn contains<T: Any + Send + Sync>(&self, key: Key<T>) -> bool {
        self.get::<T>(key).is_some()
    }

    pub fn remove<T: Any + Send + Sync>(&mut self, key: Key<T>) -> Option<Arc<T>> {
        let matches = self.contains(key);
        if matches {
            self.map
                .remove(key.name)
                .and_then(|v| v.downcast::<T>().ok())
        } else {
            None
        }
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub(crate) fn ptr_eq(&self, other: &Self) -> bool {
        if self.map.len() != other.map.len() {
            return false;
        }
        self.map.iter().all(|(k, v)| {
            other
                .map
                .get(k)
                .is_some_and(|other_v| Arc::ptr_eq(v, other_v))
        })
    }
}

impl fmt::Debug for ProviderExtensions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProviderExtensions")
            .field("keys", &self.map.keys().collect::<Vec<_>>())
            .finish()
    }
}

/// Type-indexed extension map passed to [`Scorer::score`](super::Scorer::score).
#[derive(Clone, Default)]
pub struct ScorerExtensions {
    map: HashMap<TypeId, Arc<dyn Any + Send + Sync>>,
}

impl ScorerExtensions {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    pub fn insert<T: Any + Send + Sync>(&mut self, value: T) {
        self.map.insert(TypeId::of::<T>(), Arc::new(value));
    }

    pub fn with<T: Any + Send + Sync>(mut self, value: T) -> Self {
        self.insert(value);
        self
    }

    pub fn get<T: Any + Send + Sync>(&self) -> Option<&T> {
        self.map
            .get(&TypeId::of::<T>())
            .and_then(|v| v.downcast_ref::<T>())
    }

    pub fn get_arc<T: Any + Send + Sync>(&self) -> Option<Arc<T>> {
        self.map
            .get(&TypeId::of::<T>())
            .and_then(|v| Arc::clone(v).downcast::<T>().ok())
    }

    pub fn contains<T: Any + Send + Sync>(&self) -> bool {
        self.map.contains_key(&TypeId::of::<T>())
    }

    pub fn remove<T: Any + Send + Sync>(&mut self) -> Option<Arc<T>> {
        self.map
            .remove(&TypeId::of::<T>())
            .and_then(|v| v.downcast::<T>().ok())
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }
}

impl fmt::Debug for ScorerExtensions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ScorerExtensions")
            .field("len", &self.map.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn provider_and_scorer_extensions_roundtrip() {
        const YT_ID: Key<String> = Key::new("youtube_id");
        const LRCLIB_ID: Key<u64> = Key::new("lrclib_id");

        let mut p_ext = ProviderExtensions::new()
            .with(YT_ID, "dQw4w9WgXcQ".to_string())
            .with(LRCLIB_ID, 42);
        assert_eq!(
            p_ext
                .get(Key::<String>::new("youtube_id"))
                .map(String::as_str),
            Some("dQw4w9WgXcQ")
        );
        assert_eq!(p_ext.get(LRCLIB_ID), Some(&42));
        assert!(p_ext.remove(Key::<u64>::new("youtube_id")).is_none());
        assert_eq!(p_ext.remove(LRCLIB_ID).as_deref(), Some(&42));

        let s_ext = ScorerExtensions::new().with(Duration::from_secs(180));
        assert_eq!(s_ext.get::<Duration>(), Some(&Duration::from_secs(180)));
    }
}
