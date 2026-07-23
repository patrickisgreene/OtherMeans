use bevy::math::DVec3;

/// The shape of the region around the viewer within which terrain tiles are requested to load
/// at increasingly high LOD (see `TileTree::compute_tile_distance`/`update`). A plain spherical
/// (radially symmetric) region loads/refines tiles unevenly relative to a rectangular viewport:
/// tiles near the frustum's corners sit farther from the view *position* than tiles straight
/// ahead, so under a sphere they lag behind in LOD even though they're just as visible on
/// screen. `Square` fixes that by using an axis-aligned cube-shaped region instead.
#[derive(Debug, Clone, Copy, Default)]
pub enum TerrainViewport {
    /// The original behavior: a tile is in range when its Euclidean (L2) distance to the view
    /// position is less than the threshold.
    #[default]
    Sphere,
    /// A tile is in range when it's within the threshold along *every* axis (Chebyshev/L-infinity
    /// distance) - describes an axis-aligned cube centered on the view, so refinement doesn't
    /// fall off toward the edges/corners the way a sphere's rounded boundary does.
    Square,
}

impl TerrainViewport {
    /// The "distance" from `view` to `point`, in whatever metric this viewport shape uses.
    /// Compared against a plain scalar threshold by the caller (e.g.
    /// `TileTree::compute_tile_distance`), so the threshold's own meaning (a radius for
    /// `Sphere`, a half-extent for `Square`) is entirely determined by this metric choice.
    pub fn distance(self, point: DVec3, view: DVec3) -> f64 {
        match self {
            TerrainViewport::Sphere => point.distance(view),
            TerrainViewport::Square => (point - view).abs().max_element(),
        }
    }
}
