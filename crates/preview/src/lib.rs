//! Project-scoped HTTP discovery, stable local routing, and local-only peers.
//! Application bytes use bounded multiplexed streams.
pub mod catalog;
pub mod discovery;
pub mod login;
pub mod mux;
pub mod peer;
pub mod proxy;

pub mod service;
pub use service::PreviewService;
