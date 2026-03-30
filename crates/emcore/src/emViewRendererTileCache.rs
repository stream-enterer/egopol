// SPLIT: Split from emViewRenderer.h — tile cache extracted
use crate::emCore::emImage::emImage;

/// Size of each tile in pixels.
pub const TILE_SIZE: u32 = 256;

/// A tile is a 256x256 RGBA8 bitmap.
#[derive(Clone)]
pub struct Tile {
    /// The image data for this tile.
    pub image: emImage,
    /// Whether this tile's content needs to be re-rendered.
    pub dirty: bool,
    /// LRU counter — higher means more recently used.
    pub last_used: u64,
}

impl Tile {
    pub fn new() -> Self {
        Self {
            image: emImage::new(TILE_SIZE, TILE_SIZE, 4),
            dirty: true,
            last_used: 0,
        }
    }
}

impl Default for Tile {
    fn default() -> Self {
        Self::new()
    }
}

/// Grid-based tile cache with dirty tracking and LRU eviction.
pub struct TileCache {
    /// Tiles stored by (col, row) grid position.
    tiles: Vec<Option<Tile>>,
    /// Grid dimensions.
    cols: u32,
    rows: u32,
    /// Current frame counter for LRU.
    frame_counter: u64,
    /// Maximum number of tiles to keep in memory.
    max_tiles: usize,
}

impl TileCache {
    /// Create a tile cache for a viewport of the given pixel dimensions.
    pub fn new(viewport_width: u32, viewport_height: u32, max_tiles: usize) -> Self {
        let cols = viewport_width.div_ceil(TILE_SIZE);
        let rows = viewport_height.div_ceil(TILE_SIZE);
        let count = (cols * rows) as usize;
        let mut tiles = Vec::with_capacity(count);
        tiles.resize_with(count, || None);
        Self {
            tiles,
            cols,
            rows,
            frame_counter: 0,
            max_tiles,
        }
    }

    /// Get the grid dimensions.
    pub fn grid_size(&self) -> (u32, u32) {
        (self.cols, self.rows)
    }

    /// Get a tile at the given grid position. Creates it if it doesn't exist.
    pub fn get_or_create(&mut self, col: u32, row: u32) -> &mut Tile {
        let idx = self.tile_index(col, row);
        if self.tiles[idx].is_none() {
            self.tiles[idx] = Some(Tile::new());
        }
        let tile = self.tiles[idx].as_mut().unwrap();
        tile.last_used = self.frame_counter;
        tile
    }

    /// Get a tile if it exists.
    pub fn GetRec(&self, col: u32, row: u32) -> Option<&Tile> {
        let idx = self.tile_index(col, row);
        self.tiles[idx].as_ref()
    }

    /// Mark a tile as dirty (needs re-rendering).
    pub fn mark_dirty(&mut self, col: u32, row: u32) {
        let idx = self.tile_index(col, row);
        if let Some(tile) = &mut self.tiles[idx] {
            tile.dirty = true;
        }
    }

    /// Mark all tiles as dirty.
    pub fn mark_all_dirty(&mut self) {
        for t in self.tiles.iter_mut().flatten() {
            t.dirty = true;
        }
    }

    /// Advance the frame counter and evict tiles that exceed max_tiles.
    pub fn advance_frame(&mut self) {
        self.frame_counter += 1;

        let active_count = self.tiles.iter().filter(|t| t.is_some()).count();
        if active_count <= self.max_tiles {
            return;
        }

        // Collect (index, last_used) for eviction candidates
        let mut candidates: Vec<(usize, u64)> = self
            .tiles
            .iter()
            .enumerate()
            .filter_map(|(i, t)| t.as_ref().map(|tile| (i, tile.last_used)))
            .collect();

        // Sort by last_used ascending (oldest first)
        candidates.sort_by_key(|&(_, lu)| lu);

        let to_evict = active_count - self.max_tiles;
        for &(idx, _) in candidates.iter().take(to_evict) {
            self.tiles[idx] = None;
        }
    }

    /// Get the number of active (non-None) tiles.
    pub fn active_tile_count(&self) -> usize {
        self.tiles.iter().filter(|t| t.is_some()).count()
    }

    /// Resize the cache for a new viewport size. Marks all tiles dirty.
    pub fn resize(&mut self, viewport_width: u32, viewport_height: u32) {
        self.cols = viewport_width.div_ceil(TILE_SIZE);
        self.rows = viewport_height.div_ceil(TILE_SIZE);
        let count = (self.cols * self.rows) as usize;
        self.tiles.clear();
        self.tiles.resize_with(count, || None);
    }

    fn tile_index(&self, col: u32, row: u32) -> usize {
        debug_assert!(col < self.cols && row < self.rows);
        (row * self.cols + col) as usize
    }
}
