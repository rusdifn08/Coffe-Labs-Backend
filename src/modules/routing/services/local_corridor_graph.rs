use crate::modules::{
    beans_radar::spatial_math::haversine_meters,
    routing::{dto::TravelMode, indonesia_costing::tobler_speed_kmh},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoadClass {
    Footway,
    AlleyLiving,
    ArterialOneWay,
    CollectorTwoWay,
    TollHighway,
}

#[derive(Debug, Clone, Copy)]
pub struct GraphEdge {
    pub to: usize,
    pub class: RoadClass,
    pub is_toll: bool,
    pub width_m: f64,
    pub slope: f64,
}

pub fn build_corridor_graph(origin: [f64; 2], dest: [f64; 2]) -> (Vec<[f64; 2]>, Vec<Vec<GraphEdge>>) {
    let dlat = dest[0] - origin[0];
    let dlng = dest[1] - origin[1];
    let perp = [-dlng * 0.22, dlat * 0.22];
    let nodes = vec![
        origin,
        [origin[0] + dlat * 0.35 + perp[0] * 0.15, origin[1] + dlng * 0.35 + perp[1] * 0.15],
        [origin[0] + dlat * 0.68 - perp[0] * 0.12, origin[1] + dlng * 0.68 - perp[1] * 0.12],
        [origin[0] + dlat * 0.30 + perp[0] * 0.72, origin[1] + dlng * 0.30 + perp[1] * 0.72],
        [origin[0] + dlat * 0.72 + perp[0] * 0.68, origin[1] + dlng * 0.72 + perp[1] * 0.68],
        [origin[0] + dlat * 0.32 - perp[0] * 1.15, origin[1] + dlng * 0.32 - perp[1] * 1.15],
        [origin[0] + dlat * 0.74 - perp[0] * 1.10, origin[1] + dlng * 0.74 - perp[1] * 1.10],
        [origin[0] + dlat * 0.48 - perp[0] * 1.65, origin[1] + dlng * 0.48 - perp[1] * 1.65],
        dest,
    ];
    let mut adj = vec![Vec::new(); nodes.len()];
    let add = |adj: &mut Vec<Vec<GraphEdge>>, u: usize, v: usize, class: RoadClass, oneway: bool, toll: bool, w: f64| {
        adj[u].push(GraphEdge { to: v, class, is_toll: toll, width_m: w, slope: 0.015 });
        if !oneway {
            adj[v].push(GraphEdge { to: u, class, is_toll: toll, width_m: w, slope: -0.015 });
        }
    };
    add(&mut adj, 0, 1, RoadClass::Footway, false, false, 1.2);
    add(&mut adj, 1, 2, RoadClass::Footway, false, false, 1.2);
    add(&mut adj, 2, 8, RoadClass::Footway, false, false, 1.2);
    add(&mut adj, 0, 3, RoadClass::AlleyLiving, false, false, 2.0);
    add(&mut adj, 3, 2, RoadClass::AlleyLiving, false, false, 2.0);
    add(&mut adj, 3, 4, RoadClass::CollectorTwoWay, false, false, 6.0);
    add(&mut adj, 4, 8, RoadClass::CollectorTwoWay, false, false, 6.0);
    add(&mut adj, 0, 5, RoadClass::ArterialOneWay, true, false, 8.0);
    add(&mut adj, 5, 6, RoadClass::ArterialOneWay, true, false, 8.0);
    add(&mut adj, 6, 8, RoadClass::CollectorTwoWay, false, false, 7.0);
    add(&mut adj, 5, 4, RoadClass::CollectorTwoWay, false, false, 6.5);
    add(&mut adj, 0, 7, RoadClass::TollHighway, true, true, 12.0);
    add(&mut adj, 7, 8, RoadClass::TollHighway, true, true, 12.0);
    (nodes, adj)
}

pub fn edge_cost(
    u: [f64; 2], v: [f64; 2], e: &GraphEdge, prev: Option<RoadClass>,
    mode: TravelMode, avoid_tolls: bool, mult: f64,
) -> f64 {
    let len = haversine_meters(u[0], u[1], v[0], v[1]).max(15.0);
    let base = match mode {
        TravelMode::Foot => {
            if e.is_toll || e.class == RoadClass::TollHighway { return f64::INFINITY; }
            let alpha = match e.class { RoadClass::Footway => 0.85, RoadClass::AlleyLiving => 0.95, _ => 1.35 };
            (len / (tobler_speed_kmh(e.slope) / 3.6)) * alpha
        }
        TravelMode::Motorcycle => {
            if e.is_toll || e.class == RoadClass::TollHighway || e.class == RoadClass::Footway { return f64::INFINITY; }
            let c_alley = if e.class == RoadClass::AlleyLiving { 1.05 } else { 1.0 };
            let turn = if prev.is_some() && prev != Some(e.class) { 6.0 } else { 0.0 };
            (len / 8.8) * c_alley + turn
        }
        TravelMode::Car => {
            if e.class == RoadClass::Footway || e.width_m < 2.4 || (avoid_tolls && e.is_toll) {
                return f64::INFINITY;
            }
            let c_road = if e.class == RoadClass::AlleyLiving { 1.65 } else if e.is_toll { 0.72 } else { 1.0 };
            let turn = if prev.is_some() && prev != Some(e.class) { 18.0 } else { 0.0 };
            (len / 11.2) * c_road + turn
        }
    };
    base * mult
}
