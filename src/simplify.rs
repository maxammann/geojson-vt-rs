use crate::types::VtPoint;

// square distance from a point to a segment
pub fn get_sq_seg_dist(p: VtPoint, a: VtPoint, b: VtPoint) -> f64 {
    let mut x = a.x;
    let mut y = a.y;
    let mut dx = b.x - a.x;
    let mut dy = b.y - a.y;

    if (dx != 0.0) || (dy != 0.0) {
        let t = ((p.x - a.x) * dx + (p.y - a.y) * dy) / (dx * dx + dy * dy);

        if t > 1. {
            x = b.x;
            y = b.y;
        } else if t > 0. {
            x += dx * t;
            y += dy * t;
        }
    }

    dx = p.x - x;
    dy = p.y - y;

    return dx * dx + dy * dy;
}

// calculate simplification data using optimized Douglas-Peucker algorithm
pub fn simplify(points: &mut Vec<VtPoint>, first: usize, last: usize, sq_tolerance: f64) {
    let mut max_sq_dist = sq_tolerance;
    let mut index = 0;
    let mid: i64 = first as i64 + ((last as i64 - first as i64) >> 1);
    let mut min_pos_to_mid: i64 = last as i64 - first as i64;

    for i in first + 1..last {
        let sq_dist = get_sq_seg_dist(points[i], points[first], points[last]);

        if sq_dist > max_sq_dist {
            index = i;
            max_sq_dist = sq_dist;
        } else if sq_dist == max_sq_dist {
            // a workaround to ensure we choose a pivot close to the middle of the list,
            // reducing recursion depth, for certain degenerate inputs
            // https://github.com/mapbox/geojson-vt/issues/104
            let pos_to_mid = (i as i64 - mid).abs();
            if pos_to_mid < min_pos_to_mid {
                index = i;
                min_pos_to_mid = pos_to_mid;
            }
        }
    }

    if max_sq_dist > sq_tolerance {
        // save the point importance in squared pixels as a z coordinate
        points[index].z = max_sq_dist;
        //println!("{index} - {:.70}", max_sq_dist);
        if index - first > 1 {
            simplify(points, first, index, sq_tolerance);
        }

        if last - index > 1 {
            simplify(points, index, last, sq_tolerance);
        }
    }
}

pub fn simplify_wrapper(points: &mut Vec<VtPoint>, tolerance: f64) {
    let len = points.len();

    // always retain the endpoints (1 is the max value)
    points[0].z = 1.0;
    points[len - 1].z = 1.0;

    simplify(points, 0, len - 1, tolerance * tolerance);
}
