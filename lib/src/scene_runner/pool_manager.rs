use super::object_pool::PhysicsAreaPool;

/// Statistics for a single pool - used for leak detection
#[derive(Debug, Clone, Default)]
pub struct PoolStats {
    pub name: &'static str,
    pub created: usize,
    pub in_use: usize,
    pub pooled: usize,
}

impl PoolStats {
    /// Check if stats indicate a potential leak
    /// A leak is suspected when in_use exceeds created (should never happen)
    /// or when in_use is very high with nothing pooled
    pub fn is_suspicious(&self) -> bool {
        self.in_use > self.created
    }

    /// Calculate expected total: in_use + pooled should equal created
    pub fn is_balanced(&self) -> bool {
        self.in_use + self.pooled == self.created
    }
}

/// Snapshot of all pool statistics at a point in time
#[derive(Debug, Clone, Default)]
pub struct PoolManagerSnapshot {
    pub physics_areas: PoolStats,
    pub physics_box_shapes: PoolStats,
    pub physics_sphere_shapes: PoolStats,
    // Future pools will be added here:
    // pub rendering_meshes: PoolStats,
    // pub audio_buses: PoolStats,
}

impl PoolManagerSnapshot {
    /// Check all pools for potential leaks
    pub fn find_leaks(&self) -> Vec<&PoolStats> {
        let all_stats = [
            &self.physics_areas,
            &self.physics_box_shapes,
            &self.physics_sphere_shapes,
        ];

        all_stats
            .into_iter()
            .filter(|s| s.is_suspicious() || !s.is_balanced())
            .collect()
    }

    /// Log all pool statistics
    pub fn log(&self) {
        tracing::info!(
            "[PoolManager] areas=(created={}, in_use={}, pooled={}), box_shapes=(created={}, in_use={}, pooled={}), sphere_shapes=(created={}, in_use={}, pooled={})",
            self.physics_areas.created, self.physics_areas.in_use, self.physics_areas.pooled,
            self.physics_box_shapes.created, self.physics_box_shapes.in_use, self.physics_box_shapes.pooled,
            self.physics_sphere_shapes.created, self.physics_sphere_shapes.in_use, self.physics_sphere_shapes.pooled,
        );
    }
}

/// Central manager for all object pools in the application.
///
/// This provides:
/// - Single point of access to all pools
/// - Centralized leak detection
/// - Unified statistics and logging
/// - Easy addition of new pool types without changing function signatures
pub struct PoolManager {
    /// Pool for PhysicsServer3D areas and shapes
    physics_area_pool: PhysicsAreaPool,

    // Future pools:
    // rendering_pool: RenderingServerPool,
    // audio_pool: AudioServerPool,

    /// Previous snapshot for leak detection comparison
    previous_snapshot: Option<PoolManagerSnapshot>,

    /// Frame counter for periodic health checks
    frames_since_health_check: u64,

    /// Threshold for automatic leak warnings (growth per check interval)
    leak_threshold: usize,
}

impl Default for PoolManager {
    fn default() -> Self {
        Self::new()
    }
}

impl PoolManager {
    pub fn new() -> Self {
        Self {
            physics_area_pool: PhysicsAreaPool::default(),
            previous_snapshot: None,
            frames_since_health_check: 0,
            leak_threshold: 50, // Warn if in_use grows by more than 50 between checks
        }
    }

    /// Get mutable access to the PhysicsAreaPool
    #[inline]
    pub fn physics_area(&mut self) -> &mut PhysicsAreaPool {
        &mut self.physics_area_pool
    }

    /// Get read-only access to the PhysicsAreaPool (for stats)
    #[inline]
    pub fn physics_area_ref(&self) -> &PhysicsAreaPool {
        &self.physics_area_pool
    }

    /// Take a snapshot of all pool statistics
    pub fn snapshot(&self) -> PoolManagerSnapshot {
        let (areas_created, areas_in_use, areas_pooled) = self.physics_area_pool.stats_areas();
        let (box_created, box_in_use, box_pooled) = self.physics_area_pool.stats_box_shapes();
        let (sphere_created, sphere_in_use, sphere_pooled) =
            self.physics_area_pool.stats_sphere_shapes();

        PoolManagerSnapshot {
            physics_areas: PoolStats {
                name: "physics_areas",
                created: areas_created,
                in_use: areas_in_use,
                pooled: areas_pooled,
            },
            physics_box_shapes: PoolStats {
                name: "physics_box_shapes",
                created: box_created,
                in_use: box_in_use,
                pooled: box_pooled,
            },
            physics_sphere_shapes: PoolStats {
                name: "physics_sphere_shapes",
                created: sphere_created,
                in_use: sphere_in_use,
                pooled: sphere_pooled,
            },
        }
    }

    /// Log current statistics for all pools
    pub fn log_stats(&self) {
        self.snapshot().log();
    }

    /// Perform health check and detect potential leaks.
    /// Call this periodically (e.g., every 300 frames).
    ///
    /// Returns true if potential leaks were detected.
    pub fn check_health(&mut self) -> bool {
        let current = self.snapshot();
        let mut leaks_detected = false;

        // Check for impossible states (in_use > created)
        let suspicious = current.find_leaks();
        if !suspicious.is_empty() {
            for stats in &suspicious {
                tracing::error!(
                    "[PoolManager] INVALID STATE in '{}': created={}, in_use={}, pooled={} (in_use + pooled != created)",
                    stats.name, stats.created, stats.in_use, stats.pooled
                );
            }
            leaks_detected = true;
        }

        // Compare with previous snapshot to detect growth trends
        if let Some(prev) = &self.previous_snapshot {
            // Check physics areas growth
            let areas_growth = current
                .physics_areas
                .in_use
                .saturating_sub(prev.physics_areas.in_use);
            if areas_growth > self.leak_threshold && current.physics_areas.pooled == 0 {
                tracing::warn!(
                    "[PoolManager] POTENTIAL LEAK in 'physics_areas': in_use grew by {} ({} -> {}) with pooled=0",
                    areas_growth, prev.physics_areas.in_use, current.physics_areas.in_use
                );
                leaks_detected = true;
            }

            // Check box shapes growth
            let box_growth = current
                .physics_box_shapes
                .in_use
                .saturating_sub(prev.physics_box_shapes.in_use);
            if box_growth > self.leak_threshold && current.physics_box_shapes.pooled == 0 {
                tracing::warn!(
                    "[PoolManager] POTENTIAL LEAK in 'physics_box_shapes': in_use grew by {} ({} -> {}) with pooled=0",
                    box_growth, prev.physics_box_shapes.in_use, current.physics_box_shapes.in_use
                );
                leaks_detected = true;
            }

            // Check sphere shapes growth
            let sphere_growth = current
                .physics_sphere_shapes
                .in_use
                .saturating_sub(prev.physics_sphere_shapes.in_use);
            if sphere_growth > self.leak_threshold && current.physics_sphere_shapes.pooled == 0 {
                tracing::warn!(
                    "[PoolManager] POTENTIAL LEAK in 'physics_sphere_shapes': in_use grew by {} ({} -> {}) with pooled=0",
                    sphere_growth, prev.physics_sphere_shapes.in_use, current.physics_sphere_shapes.in_use
                );
                leaks_detected = true;
            }
        }

        // Store current snapshot for next comparison
        self.previous_snapshot = Some(current);

        leaks_detected
    }

    /// Called each frame to track timing for periodic health checks.
    /// Returns true if a health check was performed this frame.
    pub fn tick(&mut self) -> bool {
        self.frames_since_health_check += 1;

        // Perform health check every 300 frames (~5 seconds at 60fps)
        if self.frames_since_health_check >= 300 {
            self.frames_since_health_check = 0;
            self.log_stats();
            self.check_health();
            return true;
        }

        false
    }

    /// Cleanup all pools, freeing all pooled resources.
    /// Call this when shutting down.
    pub fn cleanup_all(&mut self) {
        tracing::info!("[PoolManager] Cleaning up all pools...");

        // Log final stats before cleanup
        self.log_stats();

        // Cleanup physics pool
        self.physics_area_pool.cleanup();

        // Future: cleanup other pools
        // self.rendering_pool.cleanup();
        // self.audio_pool.cleanup();

        tracing::info!("[PoolManager] All pools cleaned up");
    }

    /// Set the leak detection threshold (growth per check interval that triggers warning)
    pub fn set_leak_threshold(&mut self, threshold: usize) {
        self.leak_threshold = threshold;
    }

    /// Get a summary of all pools for debugging
    pub fn debug_summary(&self) -> String {
        let snapshot = self.snapshot();
        format!(
            "PoolManager {{ areas: {}/{}/{}, box: {}/{}/{}, sphere: {}/{}/{} }}",
            snapshot.physics_areas.in_use,
            snapshot.physics_areas.pooled,
            snapshot.physics_areas.created,
            snapshot.physics_box_shapes.in_use,
            snapshot.physics_box_shapes.pooled,
            snapshot.physics_box_shapes.created,
            snapshot.physics_sphere_shapes.in_use,
            snapshot.physics_sphere_shapes.pooled,
            snapshot.physics_sphere_shapes.created,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pool_stats_balance() {
        let stats = PoolStats {
            name: "test",
            created: 10,
            in_use: 7,
            pooled: 3,
        };
        assert!(stats.is_balanced());
        assert!(!stats.is_suspicious());
    }

    #[test]
    fn test_pool_stats_unbalanced() {
        let stats = PoolStats {
            name: "test",
            created: 10,
            in_use: 8,
            pooled: 3, // 8 + 3 = 11 != 10
        };
        assert!(!stats.is_balanced());
    }

    #[test]
    fn test_pool_stats_suspicious() {
        let stats = PoolStats {
            name: "test",
            created: 10,
            in_use: 15, // More in use than created - impossible!
            pooled: 0,
        };
        assert!(stats.is_suspicious());
    }
}
