//! The floating-island candidate parcels around a set of scene parcels.

use glam::IVec2;

pub const PADDING: i32 = 2;

/// Neighbour state, in the byte order `DclFloatingIslandsManager::set_candidate_parcels` reads.
const EMPTY: u8 = 0;
const NOTHING: u8 = 1;
const LOADED: u8 = 2;

pub struct IslandLayout {
    pub min: IVec2,
    pub max: IVec2,
    pub candidates: Vec<IVec2>,
    /// `[N, S, E, W, NW, NE, SW, SE]` per candidate.
    pub corners: Vec<u8>,
}

/// `empty_center` marks the empty-parcel mode: the one parcel is surrounded by "loaded" so its
/// island has no cliffs of its own.
pub fn island_layout(scene_parcels: &[IVec2], empty_center: Option<IVec2>) -> Option<IslandLayout> {
    let first = *scene_parcels.first()?;
    let (min, max) = scene_parcels
        .iter()
        .fold((first, first), |(min, max), p| (min.min(*p), max.max(*p)));
    let bounds_min = min - IVec2::splat(PADDING);
    let bounds_max = max + IVec2::splat(PADDING);

    let occupied: std::collections::HashSet<IVec2> = scene_parcels.iter().copied().collect();
    let state_of = |p: IVec2| -> u8 {
        if p.x < bounds_min.x || p.x > bounds_max.x || p.y < bounds_min.y || p.y > bounds_max.y {
            NOTHING
        } else if occupied.contains(&p) {
            LOADED
        } else {
            EMPTY
        }
    };

    let mut candidates = Vec::new();
    let mut corners = Vec::new();
    for x in bounds_min.x..=bounds_max.x {
        for z in bounds_min.y..=bounds_max.y {
            let coord = IVec2::new(x, z);
            if occupied.contains(&coord) || empty_center == Some(coord) {
                continue;
            }
            candidates.push(coord);
            corners.extend([
                state_of(IVec2::new(x, z + 1)),
                state_of(IVec2::new(x, z - 1)),
                state_of(IVec2::new(x + 1, z)),
                state_of(IVec2::new(x - 1, z)),
                state_of(IVec2::new(x - 1, z + 1)),
                state_of(IVec2::new(x + 1, z + 1)),
                state_of(IVec2::new(x - 1, z - 1)),
                state_of(IVec2::new(x + 1, z - 1)),
            ]);
        }
    }
    if let Some(center) = empty_center {
        candidates.push(center);
        corners.extend([LOADED; 8]);
    }
    Some(IslandLayout {
        min,
        max,
        candidates,
        corners,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_scene_parcel_is_ringed_by_two_parcels_of_islands() {
        let layout = island_layout(&[IVec2::new(0, 0)], None).unwrap();
        // 5x5 square minus the scene parcel.
        assert_eq!(layout.candidates.len(), 24);
        assert_eq!(layout.corners.len(), 24 * 8);
        let beside = layout
            .candidates
            .iter()
            .position(|c| *c == IVec2::new(1, 0))
            .unwrap();
        // West of (1,0) is the scene parcel; the rest of its ring is empty.
        assert_eq!(layout.corners[beside * 8 + 3], LOADED);
        assert_eq!(layout.corners[beside * 8 + 2], EMPTY);
        let corner = layout
            .candidates
            .iter()
            .position(|c| *c == IVec2::new(2, 2))
            .unwrap();
        // The outer corner has nothing beyond it to the north and east.
        assert_eq!(layout.corners[corner * 8], NOTHING);
        assert_eq!(layout.corners[corner * 8 + 2], NOTHING);
    }

    #[test]
    fn an_empty_parcel_island_is_surrounded_by_loaded() {
        let center = IVec2::new(5, -3);
        let layout = island_layout(&[center], Some(center)).unwrap();
        assert_eq!(layout.candidates.last(), Some(&center));
        assert_eq!(&layout.corners[layout.corners.len() - 8..], &[LOADED; 8]);
        assert_eq!(layout.candidates.len(), 25);
    }

    #[test]
    fn no_parcels_have_no_layout() {
        assert!(island_layout(&[], None).is_none());
    }
}
