//! CDP-based console log streaming with level filtering.
//!
//! Connects to Steam's embedded Chromium (CEF) via Chrome DevTools Protocol
//! and streams console log events (`Runtime.consoleAPICalled`, `Log.entryAdded`).
//! Entries are batched and delivered through a callback.

mod cdp;
mod collector;

pub use collector::Collector;
