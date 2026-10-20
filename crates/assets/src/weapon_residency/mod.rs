//! Selective weapon residency: pure logic (no Bevy) deciding which weapon
//! assets must be resident for the classes in a match, and tracking their
//! preparation / pin lifetime. Integration (sink filter, sim ready table) lives elsewhere.

pub mod closure;
pub mod config;
pub mod residency;

pub use closure::{
    closure, filter_allows, ClassRequest, ClosureError, ResourceKind, WeaponClosure, WeaponDeps,
    WeaponId, WeaponRegistry,
};
pub use config::{is_active, parse, selective_active_from_env, selective_mode_from_env, SelectiveMode, ENV_VAR};
pub use residency::{Owner, RequestError, ResKey, ResState, Residency, ResidencyEvent};
