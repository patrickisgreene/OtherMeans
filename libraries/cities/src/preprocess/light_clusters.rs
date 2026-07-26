use crate::descriptor::{CityDescriptor, CityLightCluster};

const EARTH_MEAN_RADIUS_METERS: f64 = 6_371_000.0;

fn lat_lon_to_unit(lat_deg: f64, lon_deg: f64) -> [f64; 3] {
    let lat = lat_deg.to_radians();
    let lon = lon_deg.to_radians();
    let cos_lat = lat.cos();
    [cos_lat * lon.cos(), cos_lat * lon.sin(), lat.sin()]
}

fn unit_to_lat_lon(unit: [f64; 3]) -> (f64, f64) {
    let lat = unit[2].clamp(-1.0, 1.0).asin();
    let lon = unit[1].atan2(unit[0]);
    (lat.to_degrees(), lon.to_degrees())
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn normalize(v: [f64; 3]) -> [f64; 3] {
    let len = dot(v, v).sqrt();
    if len == 0.0 { v } else { [v[0] / len, v[1] / len, v[2] / len] }
}

/// Greedily merges cities within `radius_meters` of each other (great-circle distance) into
/// light clusters, seeding each cluster from the largest unassigned city so major metros anchor
/// their surrounding towns. Cluster position is the population-weighted centroid of its members.
pub fn build_light_clusters(cities: &[CityDescriptor], radius_meters: f64) -> Vec<CityLightCluster> {
    let positions: Vec<[f64; 3]> = cities
        .iter()
        .map(|city| lat_lon_to_unit(city.lat, city.lon))
        .collect();

    let cos_threshold = (radius_meters / EARTH_MEAN_RADIUS_METERS).cos();

    let mut order: Vec<usize> = (0..cities.len()).collect();
    order.sort_by(|&a, &b| cities[b].population.cmp(&cities[a].population));

    let mut assigned = vec![false; cities.len()];
    let mut clusters = Vec::new();

    for &seed in &order {
        if assigned[seed] {
            continue;
        }
        assigned[seed] = true;

        let mut members = vec![seed];
        for j in 0..cities.len() {
            if !assigned[j] && dot(positions[seed], positions[j]) >= cos_threshold {
                assigned[j] = true;
                members.push(j);
            }
        }

        let mut weighted = [0.0; 3];
        let mut total_population = 0u64;
        for &member in &members {
            let population = cities[member].population as f64;
            weighted[0] += positions[member][0] * population;
            weighted[1] += positions[member][1] * population;
            weighted[2] += positions[member][2] * population;
            total_population += cities[member].population as u64;
        }

        let centroid = if total_population > 0 {
            normalize(weighted)
        } else {
            let mut unweighted = [0.0; 3];
            for &member in &members {
                unweighted[0] += positions[member][0];
                unweighted[1] += positions[member][1];
                unweighted[2] += positions[member][2];
            }
            normalize(unweighted)
        };

        let (lat, lon) = unit_to_lat_lon(centroid);

        clusters.push(CityLightCluster {
            lat,
            lon,
            population: total_population.min(u32::MAX as u64) as u32,
            city_count: members.len().min(u16::MAX as usize) as u16,
        });
    }

    clusters
}
