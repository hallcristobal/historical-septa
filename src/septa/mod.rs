pub mod api;
pub mod train_view;
pub type FetchResult<T> = (chrono::DateTime<chrono::Utc>, T);
