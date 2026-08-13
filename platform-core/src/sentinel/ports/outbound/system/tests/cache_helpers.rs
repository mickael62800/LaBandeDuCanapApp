use super::*;

use crate::sentinel::domain::entities::system::rule::Rule;
use async_trait::async_trait;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

#[derive(Default)]
struct MemoryCache {
    data: std::sync::Mutex<std::collections::HashMap<String, String>>,
    get_calls: AtomicUsize,
    set_calls: AtomicUsize,
}

#[async_trait]
impl CachePort for MemoryCache {
    async fn get_rules(&self, _: &str) -> Result<Option<Vec<Rule>>, DomainError> {
        Ok(None)
    }
    async fn set_rules(&self, _: &str, _: &[Rule]) -> Result<(), DomainError> {
        Ok(())
    }
    async fn invalidate_rules(&self, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn get_json(&self, key: &str) -> Result<Option<String>, DomainError> {
        self.get_calls.fetch_add(1, Ordering::Relaxed);
        Ok(self.data.lock().unwrap().get(key).cloned())
    }
    async fn set_json(&self, key: &str, json: &str, _ttl: u64) -> Result<(), DomainError> {
        self.set_calls.fetch_add(1, Ordering::Relaxed);
        self.data
            .lock()
            .unwrap()
            .insert(key.to_string(), json.to_string());
        Ok(())
    }
    async fn invalidate(&self, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn invalidate_pattern(&self, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
}

#[tokio::test]
async fn cached_json_cache_miss_fetches_and_stores() {
    let cache: Arc<dyn CachePort> = Arc::new(MemoryCache::default());
    let result: Result<Vec<i32>, DomainError> =
        cached_json(&cache, "test:key", 60, || async { Ok(vec![1, 2, 3]) }).await;
    assert_eq!(result.unwrap(), vec![1, 2, 3]);
}

#[tokio::test]
async fn cached_json_cache_hit_skips_fetch() {
    let mem = Arc::new(MemoryCache::default());
    // pre-populate
    mem.data
        .lock()
        .unwrap()
        .insert("test:key".to_string(), "[9,8,7]".to_string());
    let cache: Arc<dyn CachePort> = mem;

    let fetched = AtomicUsize::new(0);
    let result: Result<Vec<i32>, DomainError> = cached_json(&cache, "test:key", 60, || async {
        fetched.fetch_add(1, Ordering::Relaxed);
        Ok(vec![0])
    })
    .await;
    assert_eq!(result.unwrap(), vec![9, 8, 7]);
    assert_eq!(fetched.load(Ordering::Relaxed), 0);
}

#[tokio::test]
async fn cached_json_invalid_json_falls_back_to_fetch() {
    let mem = Arc::new(MemoryCache::default());
    mem.data
        .lock()
        .unwrap()
        .insert("test:key".to_string(), "not-json".to_string());
    let cache: Arc<dyn CachePort> = mem;

    let result: Result<Vec<i32>, DomainError> =
        cached_json(&cache, "test:key", 60, || async { Ok(vec![42]) }).await;
    assert_eq!(result.unwrap(), vec![42]);
}
