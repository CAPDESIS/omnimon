//! OmniMon Core Library. This crate contains all the high-performance native logic, including system telemetry, network capture, and AI processing, completely decoupled from the UI.

pub mod ai;
pub mod app_icons;
pub mod audit;
pub mod audit_trail;
pub mod browser;
pub mod crypto;
pub mod killer;
pub mod metrics;
pub mod network;
mod os_native;
pub mod process_identity;
pub mod rules_engine;
pub mod security;
pub mod telemetry;
pub mod watcher;
