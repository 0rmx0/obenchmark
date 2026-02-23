//! Détection basique de l’OS à la compilation.
//! Ce module n’est utilisé que pour les constantes `GPU_BACKENDS`
//! dans les modules de benchmark (voir `benchmarks/gpu/mod.rs`).

#[cfg(target_os = "windows")]
pub const OS_NAME: &str = "windows";

#[cfg(target_os = "linux")]
pub const OS_NAME: &str = "linux";

#[cfg(target_os = "macos")]
pub const OS_NAME: &str = "macos";
