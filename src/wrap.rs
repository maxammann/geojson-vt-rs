use crate::clip::clip;
use crate::types::{for_each_point, VtFeatures, VtGeometry, VtPoint};

// Function to shift coordinates of features
pub fn shift_coords(features: &mut VtFeatures, offset: f64) {
    for feature in features {
        // TODO verify this translation
        let f = |point: &mut VtPoint| {
            point.x += offset;
        };
        for_each_point(&mut feature.geometry, f);

        feature.bbox.min.x += offset;
        feature.bbox.max.x += offset;
    }
}

// Function to wrap features around the world edges
pub fn wrap(features: &VtFeatures, buffer: f64, line_metrics: bool) -> VtFeatures {
    // left world copy
    let mut left = clip::<0>(features, -1. - buffer, buffer, -1., 2., line_metrics);
    // right world copy
    let mut right = clip::<0>(features, 1. - buffer, 2. + buffer, -1., 2., line_metrics);

    if left.is_empty() && right.is_empty() {
        return features.clone();
    }

    // center world copy
    let mut merged = clip::<0>(features, -buffer, 1. + buffer, -1., 2., line_metrics);

    if !left.is_empty() {
        // merge left into center
        shift_coords(&mut left, 1.0);
        merged.splice(0..0, left); // TODO Check if this is prepending
    }
    if !right.is_empty() {
        // merge right into center
        shift_coords(&mut right, -1.0);
        merged.extend(right);
    }
    return merged;
}
