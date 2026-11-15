#[cfg(since_530)]
pub mod asset;
#[cfg(since_647)]
pub mod defaults;
#[cfg(since_647)]
pub mod loading;
pub mod map;
#[cfg(since_530)]
pub mod map_references;
#[cfg(since_530)]
pub mod material;
#[cfg(since_647)]
pub mod shader;
#[cfg(since_647)]
mod shader_constants;
pub mod song;
#[cfg(since_647)]
pub mod wma;
#[cfg(before_530)]
pub mod worldmap;
#[cfg(all(since_530, before_647))]
pub mod worldmap_layers;
#[cfg(since_647)]
pub mod worldmap_source;
