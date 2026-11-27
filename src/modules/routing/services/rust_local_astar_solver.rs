use std::{cmp::Ordering, collections::{BinaryHeap, HashMap}};

use crate::modules::{
    beans_radar::spatial_math::haversine_meters,
    routing::{
        dto::{ModeRoutesBundleDto, RouteAlternativeDto, TravelMode, TurnStepDto},
        indonesia_costing::{
            calibrate_indonesian_eta, indonesia_timezone_info,
            MAX_ALTERNATIVE_ETA_RATIO, MAX_ALTERNATIVE_OVERLAP,
        },
        polyline6::{encode_polyline6, jaccard_overlap_ratio, polyline_length_meters},
        services::local_corridor_graph::{build_corridor_graph, edge_cost, GraphEdge, RoadClass},
    },
};

#[derive(Debug, Clone, Copy)]
struct HeapItem {
    node: usize,
    prev_edge_class: Option<RoadClass>,
    f_cost: f64,
    g_cost: f64,
}

impl Ord for HeapItem {
    fn cmp(&self, other: &Self) -> Ordering { other.f_cost.total_cmp(&self.f_cost) }
}
impl PartialOrd for HeapItem {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> { Some(self.cmp(other)) }
}
impl PartialEq for HeapItem {
    fn eq(&self, other: &Self) -> bool { self.node == other.node && self.f_cost == other.f_cost }
}
impl Eq for HeapItem {}

/// Built-in Native Rust Edge-Based A* Solver dengan 3 fungsi biaya fisika terpisah (Foot / Motorcycle / Car).
pub fn solve_local_multimodal_astar(
    origin: [f64; 2], dest: [f64; 2], mode: TravelMode, avoid_tolls: bool,
) -> ModeRoutesBundleDto {
    let (nodes, edges) = build_corridor_graph(origin, dest);
    let primary_path = run_astar(&nodes, &edges, 0, nodes.len() - 1, mode, avoid_tolls, &HashMap::new());
    let mut penalty_map = HashMap::new();
    for pair in primary_path.windows(2) {
        penalty_map.insert((pair[0], pair[1]), 1.65);
    }
    let alt_path = run_astar(&nodes, &edges, 0, nodes.len() - 1, mode, avoid_tolls, &penalty_map);

    let primary = build_dto(&nodes, &edges, &primary_path, mode, avoid_tolls, 0, "Rute Terbaik");
    let mut alternatives = Vec::new();
    if alt_path.len() >= 2 && alt_path != primary_path {
        let alt = build_dto(&nodes, &edges, &alt_path, mode, avoid_tolls, 1, "Alternatif 1");
        if jaccard_overlap_ratio(&primary.coordinates, &alt.coordinates) <= MAX_ALTERNATIVE_OVERLAP
            && alt.duration_seconds <= primary.duration_seconds * MAX_ALTERNATIVE_ETA_RATIO
        {
            alternatives.push(alt);
        }
    }
    ModeRoutesBundleDto { mode, primary, alternatives }
}

fn run_astar(
    nodes: &[[f64; 2]], adj: &[Vec<GraphEdge>], start: usize, goal: usize,
    mode: TravelMode, avoid_tolls: bool, pen: &HashMap<(usize, usize), f64>,
) -> Vec<usize> {
    let mut best_g = vec![f64::INFINITY; nodes.len()];
    let mut parent = vec![None; nodes.len()];
    let mut heap = BinaryHeap::new();
    best_g[start] = 0.0;
    heap.push(HeapItem {
        node: start, prev_edge_class: None, g_cost: 0.0,
        f_cost: haversine_meters(nodes[start][0], nodes[start][1], nodes[goal][0], nodes[goal][1]) / mode.max_speed_mps(),
    });
    while let Some(cur) = heap.pop() {
        if cur.node == goal { break; }
        if cur.g_cost > best_g[cur.node] { continue; }
        for e in &adj[cur.node] {
            let mult = pen.get(&(cur.node, e.to)).copied().unwrap_or(1.0);
            let step = edge_cost(nodes[cur.node], nodes[e.to], e, cur.prev_edge_class, mode, avoid_tolls, mult);
            if !step.is_finite() { continue; }
            let next_g = cur.g_cost + step;
            if next_g < best_g[e.to] {
                best_g[e.to] = next_g;
                parent[e.to] = Some(cur.node);
                let h = haversine_meters(nodes[e.to][0], nodes[e.to][1], nodes[goal][0], nodes[goal][1]) / mode.max_speed_mps();
                heap.push(HeapItem { node: e.to, prev_edge_class: Some(e.class), g_cost: next_g, f_cost: next_g + h });
            }
        }
    }
    let mut path = Vec::new();
    let mut curr = Some(goal);
    while let Some(n) = curr { path.push(n); curr = parent[n]; }
    path.reverse();
    if path.first() == Some(&start) { path } else { vec![start, goal] }
}

fn build_dto(
    nodes: &[[f64; 2]], adj: &[Vec<GraphEdge>], path: &[usize], mode: TravelMode, avoid_tolls: bool, idx: usize, label: &str,
) -> RouteAlternativeDto {
    let coords: Vec<[f64; 2]> = path.iter().filter_map(|&i| nodes.get(i).copied()).collect();
    let dist_m = polyline_length_meters(&coords);
    let mut has_toll = false;
    let mut uses_alleys = false;
    for pair in path.windows(2) {
        if let Some(edge) = adj.get(pair[0]).and_then(|es| es.iter().find(|e| e.to == pair[1])) {
            if edge.is_toll { has_toll = true; }
            if edge.class == RoadClass::Footway || edge.class == RoadClass::AlleyLiving { uses_alleys = true; }
        }
    }
    let raw_s = dist_m / match mode { TravelMode::Foot => 1.33, TravelMode::Motorcycle => 8.8, TravelMode::Car => 7.4 };
    let (_, is_rush, is_day) = indonesia_timezone_info(coords.first().map(|p| p[1]).unwrap_or(107.6));
    let (dur_s, mult) = calibrate_indonesian_eta(mode, raw_s, dist_m, has_toll, is_rush, is_day, path.len().saturating_sub(1));
    let steps = path.windows(2).enumerate().map(|(i, w)| {
        let seg_d = haversine_meters(nodes[w[0]][0], nodes[w[0]][1], nodes[w[1]][0], nodes[w[1]][1]);
        TurnStepDto {
            instruction: if i == 0 { "Mulai ikuti jalur A* optimal".to_string() } else { "Ikuti segmen jalur berikutnya".to_string() },
            distance_meters: seg_d,
            duration_seconds: seg_d / mode.max_speed_mps().max(1.3),
            maneuver_type: "continue".to_string(),
            street_name: "Jalur Lokal".to_string(),
            lat: nodes[w[0]][0],
            lng: nodes[w[0]][1],
        }
    }).collect();
    RouteAlternativeDto {
        route_id: format!("rust_astar_{}_{idx}", mode.cache_key_part()),
        label: label.to_string(),
        provider: "rust_local_astar".to_string(),
        mode,
        cache_hit: false,
        distance_meters: (dist_m * 10.0).round() / 10.0,
        duration_seconds: dur_s,
        base_duration_seconds: raw_s.round(),
        traffic_multiplier: mult,
        has_toll,
        avoided_tolls: avoid_tolls || mode != TravelMode::Car,
        uses_alleys,
        mode_badge: mode.default_badge(avoid_tolls).to_string(),
        polyline6: encode_polyline6(&coords),
        coordinates: coords,
        steps,
    }
}
