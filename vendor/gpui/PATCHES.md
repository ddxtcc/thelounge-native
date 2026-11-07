# Local GPUI patches

- `src/platform/blade/blade_atlas.rs`: drop pending texture initializations and uploads when a texture is destroyed, and skip missing textures during flush.
- `src/taffy.rs`: use explicit `f32` literals to avoid the future-incompatibility warning.
