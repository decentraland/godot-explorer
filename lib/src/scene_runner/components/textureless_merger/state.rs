//! Per-Scene state for the textureless merger.

use std::collections::{HashMap, HashSet};

use godot::classes::MeshInstance3D;
use godot::prelude::*;

use crate::dcl::components::SceneEntityId;

use super::cell_grid::BucketKey;
use super::combiner::MeshPart;
use super::metrics::MergerStats;

/// One bucket = (transparency, cull_mode, cell_x, cell_z). Accumulates
/// mergeable parts until it's flushed into a combined `MeshInstance3D`.
#[derive(Default)]
pub struct BucketBuilder {
    pub parts: Vec<MeshPart>,
    /// `true` once a merged node has been spawned and the originals have
    /// been queued for suppression. Late-arriving parts in the same bucket
    /// stay standalone for now (re-flush is future work).
    pub flushed: bool,
}

#[derive(Default)]
pub struct TexturelessMergerState {
    pub stats: MergerStats,
    /// Entities the classifier has already inspected at least once. Used to
    /// avoid re-walking the same GLTF every time the entity bounces back to
    /// `pending_textureless_promotion` (e.g. via a transform write).
    pub classified: HashSet<SceneEntityId>,

    /// Active buckets keyed by `(transparency, cull_mode, cx, cz)`.
    pub cell_buckets: HashMap<BucketKey, BucketBuilder>,
    /// Spawned merged nodes, kept so a future demote / re-flush can find
    /// them by `BucketKey`.
    pub merged_nodes: HashMap<BucketKey, Gd<MeshInstance3D>>,
    /// Source `MeshInstance3D`s queued to have their `mesh` set to None on
    /// the next call. Two-frame split prevents a one-frame visual gap
    /// between the merged node appearing and the originals disappearing.
    pub pending_suppress: Vec<Gd<MeshInstance3D>>,

    /// Sum of vertex counts across all flushed buckets — debug only.
    pub merged_vertex_total: u64,
    /// Number of buckets that reached MIN_BUCKET_SIZE and got flushed.
    pub buckets_flushed: u32,
    /// Sources whose `mesh` was nulled. Proxy for "draws saved".
    pub originals_suppressed: u32,
}
