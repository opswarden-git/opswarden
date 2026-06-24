// --- server/src/app/gif/search_gifs.rs ---
//
// Validate and run an external GIF search. Caps the limit, defaults/validates
// the rating, and rejects a blank or oversized query — then delegates to the
// `GifSearch` port (GIPHY in production).

use std::sync::Arc;

use crate::domain::error::DomainError;
use crate::ports::{GifResult, GifSearch};

const DEFAULT_LIMIT: u32 = 12;
const MAX_LIMIT: u32 = 20;
const MAX_QUERY_LEN: usize = 100;

pub struct SearchGifsCommand {
    pub query: String,
    pub limit: Option<u32>,
    pub rating: Option<String>,
}

pub struct SearchGifsUseCase {
    gifs: Arc<dyn GifSearch>,
}

impl SearchGifsUseCase {
    pub fn new(gifs: Arc<dyn GifSearch>) -> Self {
        Self { gifs }
    }

    pub async fn search(&self, cmd: SearchGifsCommand) -> Result<Vec<GifResult>, DomainError> {
        let query = cmd.query.trim();
        if query.is_empty() || query.len() > MAX_QUERY_LEN {
            return Err(DomainError::InvalidGifQuery);
        }
        // Clamp the limit to a sane window; only `g`/`pg` ratings are allowed
        // (default `pg`), so a crafted `rating` can never widen the content.
        let limit = cmd.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
        let rating = match cmd.rating.as_deref() {
            Some("g") => "g",
            _ => "pg",
        };
        self.gifs.search(query, limit, rating).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::sync::Mutex;

    #[derive(Default)]
    struct MockGifSearch {
        calls: Mutex<Vec<(String, u32, String)>>,
    }

    #[async_trait]
    impl GifSearch for MockGifSearch {
        async fn search(
            &self,
            query: &str,
            limit: u32,
            rating: &str,
        ) -> Result<Vec<GifResult>, DomainError> {
            self.calls
                .lock()
                .unwrap()
                .push((query.to_string(), limit, rating.to_string()));
            Ok(vec![GifResult {
                id: "1".into(),
                title: "t".into(),
                url: "u".into(),
                preview_url: "p".into(),
                width: 1,
                height: 1,
            }])
        }
    }

    fn cmd(query: &str, limit: Option<u32>, rating: Option<&str>) -> SearchGifsCommand {
        SearchGifsCommand {
            query: query.to_string(),
            limit,
            rating: rating.map(String::from),
        }
    }

    #[tokio::test]
    async fn a_blank_query_is_rejected() {
        let mock = Arc::new(MockGifSearch::default());
        let uc = SearchGifsUseCase::new(mock.clone());
        assert_eq!(
            uc.search(cmd("   ", None, None)).await.unwrap_err(),
            DomainError::InvalidGifQuery
        );
        assert!(mock.calls.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn an_oversized_query_is_rejected() {
        let mock = Arc::new(MockGifSearch::default());
        let uc = SearchGifsUseCase::new(mock);
        let long = "x".repeat(101);
        assert_eq!(
            uc.search(cmd(&long, None, None)).await.unwrap_err(),
            DomainError::InvalidGifQuery
        );
    }

    #[tokio::test]
    async fn it_defaults_limit_and_rating_and_trims_the_query() {
        let mock = Arc::new(MockGifSearch::default());
        let uc = SearchGifsUseCase::new(mock.clone());
        uc.search(cmd("  deploy  ", None, None)).await.unwrap();
        assert_eq!(
            mock.calls.lock().unwrap().as_slice(),
            &[("deploy".to_string(), 12, "pg".to_string())]
        );
    }

    #[tokio::test]
    async fn it_caps_the_limit_and_accepts_g_rating() {
        let mock = Arc::new(MockGifSearch::default());
        let uc = SearchGifsUseCase::new(mock.clone());
        uc.search(cmd("deploy", Some(999), Some("g")))
            .await
            .unwrap();
        assert_eq!(
            mock.calls.lock().unwrap().as_slice(),
            &[("deploy".to_string(), 20, "g".to_string())]
        );
    }

    #[tokio::test]
    async fn an_unknown_rating_falls_back_to_pg() {
        let mock = Arc::new(MockGifSearch::default());
        let uc = SearchGifsUseCase::new(mock.clone());
        uc.search(cmd("deploy", Some(5), Some("r"))).await.unwrap();
        assert_eq!(mock.calls.lock().unwrap()[0].2, "pg");
    }
}
