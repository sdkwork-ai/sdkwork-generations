//! API assembly for sdkwork-generations.
//! Application bootstrap lives in `bootstrap.rs`; route inventory is in `assembly-manifest.json`.
// SDKWORK-ASSEMBLY-LIB-CUSTOM

mod bootstrap;
mod generated;

pub use bootstrap::{app_api_route_manifest, ApiAssembly, assemble_api_router, assemble_api_router_with_pool, assemble_app_api_contribution, bootstrap_database_from_env, web_module, web_module_with_pool};

pub fn assembly_route_count() -> usize {
    generated::ROUTE_CRATE_COUNT
}