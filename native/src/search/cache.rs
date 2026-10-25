use crate::model::SearchResponse;
use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

const CAPACITY: usize = 64;
const TTL: Duration = Duration::from_secs(300);

#[derive(Default)]
pub(super) struct SearchCache {
    entries: HashMap<String, (Instant, SearchResponse)>,
}

impl SearchCache {
    pub fn get(&mut self, key: &str, now: Instant) -> Option<SearchResponse> {
        let (inserted, response) = self.entries.get(key)?;
        if now.saturating_duration_since(*inserted) < TTL {
            Some(response.clone())
        } else {
            self.entries.remove(key);
            None
        }
    }

    pub fn insert(&mut self, key: String, response: SearchResponse, now: Instant) {
        self.entries
            .retain(|_, (inserted, _)| now.saturating_duration_since(*inserted) < TTL);
        if self.entries.len() >= CAPACITY && !self.entries.contains_key(&key) {
            let oldest = self
                .entries
                .iter()
                .min_by_key(|(_, (inserted, _))| *inserted)
                .map(|(key, _)| key.clone());
            if let Some(oldest) = oldest {
                self.entries.remove(&oldest);
            }
        }
        self.entries.insert(key, (now, response));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expires_entries_and_reclaims_them_on_insert() {
        let mut cache = SearchCache::default();
        let now = Instant::now();
        cache.insert("one".into(), SearchResponse::default(), now);
        assert!(
            cache
                .get("one", now + TTL - Duration::from_millis(1))
                .is_some()
        );
        assert!(cache.get("one", now + TTL).is_none());
        cache.insert("old".into(), SearchResponse::default(), now);
        cache.insert("new".into(), SearchResponse::default(), now + TTL);
        assert_eq!(cache.entries.len(), 1);
        assert!(cache.get("new", now + TTL).is_some());
    }

    #[test]
    fn evicts_oldest_entry_and_allows_replacement_at_capacity() {
        let mut cache = SearchCache::default();
        let now = Instant::now();
        for index in 0..CAPACITY {
            cache.insert(
                index.to_string(),
                SearchResponse::default(),
                now + Duration::from_millis(index as u64),
            );
        }
        let later = now + Duration::from_secs(1);
        cache.insert("1".into(), SearchResponse::default(), later);
        assert_eq!(cache.entries.len(), CAPACITY);
        cache.insert("overflow".into(), SearchResponse::default(), later);
        assert_eq!(cache.entries.len(), CAPACITY);
        assert!(cache.get("0", later).is_none());
        assert!(cache.get("1", later).is_some());
        assert!(cache.get("overflow", later).is_some());
    }
}
