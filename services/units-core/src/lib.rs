//! UNITS Core Service Library
//!
//! This library provides the core service layer for the UNITS system,
//! including transaction processing, object management, and proof generation.

pub mod config;
pub mod error;
pub mod json_rpc;
pub mod server;
pub mod service;
pub mod services;

// Re-export commonly used types
pub use config::Config;
pub use error::{ServiceError, ServiceResult};
pub use service::UnitsService;
pub use services::{MinimalServiceFactory, MinimalServiceContainer};