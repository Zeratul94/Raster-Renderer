# Project Architecture & Conventions

## Material System
- **Global Materials**: Materials are stored in a central `MaterialLibrary` (a global vector) rather than within individual `Mesh` structs.
- **Mesh Indices**: Meshes store indices (`matIdcs`) that point into the global materials vector.
- **Deduplication**: Use a `HashMap` (e.g., in `MaterialLibrary`) during loading to deduplicate materials by name (e.g., from `.mtl` files).

## Engine Decoupling
- **Camera & Surface**: The `Camera` struct should be decoupled from the `Surface` (render target). The `Camera` handles projection and frustum logic, while the `Surface` handles the actual rasterization and pixel buffer management. This avoids unnecessary lifetime dependencies in the engine structs.
