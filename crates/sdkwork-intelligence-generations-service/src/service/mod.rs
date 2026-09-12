//! Service module root.

pub mod generations_service;
pub mod handlers;

pub use generations_service::{GenerationsService, GenerationsServiceState, build_app_routes};

#[cfg(test)]
mod lifecycle_tests;
