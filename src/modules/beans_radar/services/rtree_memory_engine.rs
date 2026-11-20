use std::sync::{
    atomic::{AtomicI64, AtomicUsize, Ordering},
    Arc,
};
use arc_swap::ArcSwap;
use chrono::Utc;
use rstar::RTree;
use sqlx::PgPool;
use tracing::{info, warn};
use uuid::Uuid;

use crate::modules::beans_radar::{
    models::CompactSpatialNode,
    repositories::{favorite_place_repo, radar_place_repo},
    spatial_math::{chord_dist_sq_for_meters, haversine_meters, to_unit_sphere},
};

#[derive(Debug, Clone)]
pub struct RTreeCandidate {
    pub place_id: Uuid,
    pub distance_meters: f64,
}

#[derive(Debug, Clone)]
pub struct RTreeQueryFilter {
    pub category_code: Option<u8>,
    pub min_rating_x10: u8,
    pub sells_beans_only: bool,
}

/// Mesin spasial L1 Lock-Free In-Memory R*-Tree berbasis proyeksi Unit Sphere 3D.
pub struct RTreeMemoryEngine {
    tree: ArcSwap<RTree<CompactSpatialNode>>,
    node_count: AtomicUsize,
    last_refresh_epoch: AtomicI64,
}

impl RTreeMemoryEngine {
    pub fn empty() -> Arc<Self> {
        Arc::new(Self {
            tree: ArcSwap::from_pointee(RTree::new()),
            node_count: AtomicUsize::new(0),
            last_refresh_epoch: AtomicI64::new(0),
        })
    }

    pub fn loaded_nodes_count(&self) -> usize {
        self.node_count.load(Ordering::Relaxed)
    }

    /// Memuat ulang seluruh titik aktif dari NeonDB dan membangun ulang R*-Tree via `bulk_load`.
    pub async fn reload_from_db(&self, pool: &PgPool) -> usize {
        let _ = favorite_place_repo::sync_published_business_roasteries(pool).await;
        match radar_place_repo::load_all_compact_nodes(pool).await {
            Ok(rows) => {
                let nodes: Vec<CompactSpatialNode> =
                    rows.iter().map(CompactSpatialNode::from_compact_row).collect();
                let count = nodes.len();
                let new_tree = RTree::bulk_load(nodes);
                self.tree.store(Arc::new(new_tree));
                self.node_count.store(count, Ordering::Relaxed);
                self.last_refresh_epoch
                    .store(Utc::now().timestamp(), Ordering::Relaxed);
                info!(count, "🗺️ Beans Radar L1 R*-Tree dimuat ulang dengan proyeksi 3D Unit Sphere.");
                count
            }
            Err(err) => {
                warn!(error = %err, "Gagal memuat ulang R*-Tree Beans Radar dari NeonDB.");
                self.loaded_nodes_count()
            }
        }
    }

    /// Menjalankan auto-refresh R*-Tree setiap 5 menit di background tanpa blocking request baca.
    pub fn spawn_background_refresher(engine: Arc<Self>, pool: PgPool) {
        tokio::spawn(async move {
            engine.reload_from_db(&pool).await;
            let mut ticker = tokio::time::interval(std::time::Duration::from_secs(300));
            ticker.tick().await;
            loop {
                ticker.tick().await;
                engine.reload_from_db(&pool).await;
            }
        });
    }

    /// Mengembalikan kandidat terurut berjarak <= `radius_km` dalam < 0.1 ms menggunakan `locate_within_distance`.
    pub fn locate_within_radius(
        &self,
        lat: f64,
        lng: f64,
        radius_km: f64,
        filter: &RTreeQueryFilter,
        max_candidates: usize,
    ) -> Vec<RTreeCandidate> {
        let guard = self.tree.load();
        if guard.size() == 0 {
            return Vec::new();
        }
        let query_pt = to_unit_sphere(lat, lng);
        let max_chord_sq = chord_dist_sq_for_meters(radius_km * 1000.0);

        let mut matched: Vec<RTreeCandidate> = guard
            .locate_within_distance(query_pt, max_chord_sq)
            .filter(|node| Self::matches_filter(node, filter))
            .map(|node| RTreeCandidate {
                place_id: node.place_id,
                distance_meters: haversine_meters(lat, lng, node.latitude, node.longitude),
            })
            .collect();

        matched.sort_by(|a, b| a.distance_meters.total_cmp(&b.distance_meters));
        matched.truncate(max_candidates);
        matched
    }

    /// Pencarian K-Nearest Neighbors global se-Indonesia saat pengguna meminta mode terdekat tanpa batas radius ketat.
    pub fn knn_nearest(
        &self,
        lat: f64,
        lng: f64,
        filter: &RTreeQueryFilter,
        k: usize,
    ) -> Vec<RTreeCandidate> {
        let guard = self.tree.load();
        if guard.size() == 0 {
            return Vec::new();
        }
        let query_pt = to_unit_sphere(lat, lng);
        guard
            .nearest_neighbor_iter(query_pt)
            .filter(|node| Self::matches_filter(node, filter))
            .take(k)
            .map(|node| RTreeCandidate {
                place_id: node.place_id,
                distance_meters: haversine_meters(lat, lng, node.latitude, node.longitude),
            })
            .collect()
    }

    fn matches_filter(node: &CompactSpatialNode, filter: &RTreeQueryFilter) -> bool {
        if let Some(cat_code) = filter.category_code {
            if node.category_code != cat_code {
                return false;
            }
        }
        if node.rating_x10 < filter.min_rating_x10 {
            return false;
        }
        if filter.sells_beans_only && !node.sells_roast_beans {
            return false;
        }
        true
    }
}
