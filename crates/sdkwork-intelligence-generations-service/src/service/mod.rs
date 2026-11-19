//! Service module root.

pub mod generations_service;
pub mod handlers;

pub use generations_service::{build_app_routes, GenerationsService, GenerationsServiceState};

#[cfg(test)]
mod lifecycle_tests;
