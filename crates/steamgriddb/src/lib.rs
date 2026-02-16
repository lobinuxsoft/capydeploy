//! SteamGridDB API client for artwork search and download.
//!
//! Provides an async client for the [SteamGridDB](https://www.steamgriddb.com)
//! API v2 with optional disk caching for downloaded images.

pub mod cache;
pub mod client;
pub mod types;

pub use client::Client;
pub use types::{ImageData, ImageFilters, SearchResult};
