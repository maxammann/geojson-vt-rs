use crate::Features;

// Function to shift coordinates of features
pub fn shift_coords(features: &mut Features, offset: f64) {
    // for (auto& feature : features) {
    //     mapbox::geometry::for_each_point(feature.geometry,
    //                                      [offset](vt_point& point) { point.x += offset; });
    //     feature.bbox.min.x += offset;
    //     feature.bbox.max.x += offset;
    // }
}

// Function to wrap features around the world edges
pub fn wrap(features: &Features, buffer: f64, line_metrics: bool) -> Features {
    // // left world copy
    // auto left = clip<0>(features, -1 - buffer, buffer, -1, 2, line_metrics);
    // // right world copy
    // auto right = clip<0>(features, 1 - buffer, 2 + buffer, -1, 2, line_metrics);

    // if (left.empty() && right.empty())
    //     return features;

    // // center world copy
    // auto merged = clip<0>(features, -buffer, 1 + buffer, -1, 2, line_metrics);

    // if (!left.empty()) {
    //     // merge left into center
    //     shift_coords(left, 1.0);
    //     merged.insert(merged.begin(), left.begin(), left.end());
    // }
    // if (!right.empty()) {
    //     // merge right into center
    //     shift_coords(right, -1.0);
    //     merged.insert(merged.end(), right.begin(), right.end());
    // }
    // return merged;

    unimplemented!()
}
