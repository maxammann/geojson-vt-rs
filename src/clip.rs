use crate::types::*;

pub struct Clipper<const I: usize> {
    k1: f64,
    k2: f64,
    line_metrics: bool,
}

impl<const I: usize> Clipper<I> {
    pub fn new(k1: f64, k2: f64, line_metrics: bool) -> Self {
        Clipper {
            k1,
            k2,
            line_metrics,
        }
    }

    pub fn clip_empty(&self) -> VtGeometry {
        VtGeometry::Empty(())
    }

    pub fn clip_point(&self, point: &VtPoint) -> VtGeometry {
        VtGeometry::Point(*point)
    }

    pub fn clip_multi_point(&self, points: &VtMultiPoint) -> VtGeometry {
        let mut part: VtMultiPoint = Vec::new();
        for p in points {
            let ak = GetCoordinate::<I>::get(p);
            if ak >= self.k1 && ak <= self.k2 {
                part.push(*p);
            }
        }
        VtGeometry::MultiPoint(part)
    }

    pub fn clip_line_string(&self, line: &VtLineString) -> VtGeometry {
        let mut parts: VtMultiLineString = VtMultiLineString::new();
        self.clip_line(line, &mut parts);

        if parts.len() == 1 {
            return VtGeometry::LineString(parts[0].clone());
        }

        VtGeometry::MultiLineString(parts.clone())
    }

    pub fn clip_multi_line_string(&self, lines: &VtMultiLineString) -> VtGeometry {
        let mut parts: VtMultiLineString = Vec::new();
        for line in lines {
            self.clip_line(line, &mut parts);
        }

        if parts.len() == 1 {
            return VtGeometry::LineString(parts[0].clone());
        }

        VtGeometry::MultiLineString(parts.clone())
    }

    pub fn clip_polygon(&self, polygon: &VtPolygon) -> VtGeometry {
        let mut result: VtPolygon = Vec::new();
        for ring in polygon {
            let new_ring = self.clip_ring(ring);
            if !new_ring.elements.is_empty() {
                result.push(new_ring);
            }
        }
        VtGeometry::Polygon(result)
    }

    pub fn clip_multi_polygon(&self, polygons: &VtMultiPolygon) -> VtGeometry {
        let mut result: VtMultiPolygon = Vec::new();
        for polygon in polygons {
            let mut p: VtPolygon = Vec::new();
            for ring in polygon {
                let new_ring = self.clip_ring(ring);
                if !new_ring.elements.is_empty() {
                    p.push(new_ring);
                }
            }
            if !p.is_empty() {
                result.push(p.clone());
            }
        }
        VtGeometry::MultiPolygon(result)
    }

    pub fn clip_geometry_collection(&self, geometries: &VtGeometryCollection) -> VtGeometry {
        let mut result: VtGeometryCollection = Vec::new();
        for geometry in geometries {
            // TODO: verify if translated correctly
            result.push(self.clip_geometry(geometry));
        }
        VtGeometry::GeometryCollection(result)
    }
    fn clip_geometry(&self, geometry: &VtGeometry) -> VtGeometry {
        match geometry {
            VtGeometry::Empty(_empty) => self.clip_empty(),
            VtGeometry::Point(point) => self.clip_point(point),
            VtGeometry::MultiPoint(multi_point) => self.clip_multi_point(multi_point),
            VtGeometry::LineString(line_string) => self.clip_line_string(line_string),
            VtGeometry::MultiLineString(multi_line_string) => {
                self.clip_multi_line_string(multi_line_string)
            }
            VtGeometry::Polygon(polygon) => self.clip_polygon(polygon),
            VtGeometry::MultiPolygon(multi_polygon) => self.clip_multi_polygon(multi_polygon),
            VtGeometry::GeometryCollection(geometry_collection) => {
                self.clip_geometry_collection(geometry_collection)
            }
        }
    }
}

impl<const I: usize> Clipper<I> {
    fn new_slice(&self, line: &VtLineString) -> VtLineString {
        let mut slice: VtLineString = VtLineString {
            dist: line.dist,
            ..Default::default()
        };

        if self.line_metrics {
            slice.seg_start = line.seg_start;
            slice.seg_end = line.seg_end;
        }
        slice
    }

    fn clip_line(&self, line: &VtLineString, slices: &mut Vec<VtLineString>) {
        let len = line.elements.len();
        let mut line_len = line.seg_start;
        let mut seg_len = 0.0;
        let mut t;

        if len < 2 {
            return;
        }

        let mut slice = self.new_slice(line);

        for i in 0..(len - 1) {
            let a = line.elements[i];
            let b = line.elements[i + 1];
            let ak = GetCoordinate::<I>::get(&a);
            let bk = GetCoordinate::<I>::get(&b);
            let is_last_seg = i == (len - 2);

            if self.line_metrics {
                seg_len = (b.x - a.x).hypot(b.y - a.y);
            }

            if ak < self.k1 {
                if bk > self.k2 {
                    // ---|-----|-->
                    t = calc_progress::<I>(&a, &b, self.k1);
                    slice.elements.push(intersect::<I>(&a, &b, self.k1, t));
                    if self.line_metrics {
                        slice.seg_start = line_len + seg_len * t;
                    }

                    t = calc_progress::<I>(&a, &b, self.k2);
                    slice.elements.push(intersect::<I>(&a, &b, self.k2, t));
                    if self.line_metrics {
                        slice.seg_end = line_len + seg_len * t;
                    }
                    slices.push(slice);

                    slice = self.new_slice(line);
                } else if bk > self.k1 {
                    // ---|-->  |
                    t = calc_progress::<I>(&a, &b, self.k1);
                    slice.elements.push(intersect::<I>(&a, &b, self.k1, t));
                    if self.line_metrics {
                        slice.seg_start = line_len + seg_len * t;
                    }
                    if is_last_seg {
                        slice.elements.push(b); // last point
                    }
                } else if bk == self.k1 && !is_last_seg {
                    // --->|..  |
                    if self.line_metrics {
                        slice.seg_start = line_len + seg_len;
                    }
                    slice.elements.push(b);
                }
            } else if ak > self.k2 {
                if bk < self.k1 {
                    // <--|-----|---
                    t = calc_progress::<I>(&a, &b, self.k2);
                    slice.elements.push(intersect::<I>(&a, &b, self.k2, t));
                    if self.line_metrics {
                        slice.seg_start = line_len + seg_len * t;
                    }

                    t = calc_progress::<I>(&a, &b, self.k1);
                    slice.elements.push(intersect::<I>(&a, &b, self.k1, t));
                    if self.line_metrics {
                        slice.seg_end = line_len + seg_len * t;
                    }

                    slices.push(slice);

                    slice = self.new_slice(line);
                } else if bk < self.k2 {
                    // |  <--|---
                    t = calc_progress::<I>(&a, &b, self.k2);
                    slice.elements.push(intersect::<I>(&a, &b, self.k2, t));
                    if self.line_metrics {
                        slice.seg_start = line_len + seg_len * t;
                    }
                    if is_last_seg {
                        slice.elements.push(b); // last point
                    }
                } else if bk == self.k2 && !is_last_seg {
                    // |  ..|<---
                    if self.line_metrics {
                        slice.seg_start = line_len + seg_len;
                    }
                    slice.elements.push(b);
                }
            } else {
                slice.elements.push(a);

                if bk < self.k1 {
                    // <--|---  |
                    t = calc_progress::<I>(&a, &b, self.k1);
                    slice.elements.push(intersect::<I>(&a, &b, self.k1, t));
                    if self.line_metrics {
                        slice.seg_end = line_len + seg_len * t;
                    }
                    slices.push(slice);
                    slice = self.new_slice(line);
                } else if bk > self.k2 {
                    // |  ---|-->
                    t = calc_progress::<I>(&a, &b, self.k2);
                    slice.elements.push(intersect::<I>(&a, &b, self.k2, t));
                    if self.line_metrics {
                        slice.seg_end = line_len + seg_len * t;
                    }
                    slices.push(slice);
                    slice = self.new_slice(line);
                } else if is_last_seg {
                    // | --> |
                    slice.elements.push(b);
                }
            }

            if self.line_metrics {
                line_len += seg_len;
            }
        }

        if !slice.elements.is_empty() {
            // add the final slice
            if self.line_metrics {
                slice.seg_end = line_len;
            }
            slices.push(slice);
        }
    }

    // Mimic the clipRing function from C++
    fn clip_ring(&self, ring: &VtLinearRing) -> VtLinearRing {
        let len = ring.elements.len();
        let mut slice = VtLinearRing {
            area: ring.area,
            ..Default::default()
        };

        if len < 2 {
            return slice;
        }

        for i in 0..(len - 1) {
            let a = ring.elements[i];
            let b = ring.elements[i + 1];
            let ak = GetCoordinate::<I>::get(&a);
            let bk = GetCoordinate::<I>::get(&b);

            if ak < self.k1 {
                if bk > self.k1 {
                    // ---|-->  |
                    slice.elements.push(intersect::<I>(
                        &a,
                        &b,
                        self.k1,
                        calc_progress::<I>(&a, &b, self.k1),
                    ));
                    if bk > self.k2 {
                        // ---|-----|-->
                        slice.elements.push(intersect::<I>(
                            &a,
                            &b,
                            self.k2,
                            calc_progress::<I>(&a, &b, self.k2),
                        ));
                    } else if i == len - 2 {
                        slice.elements.push(b); // last point
                    }
                }
            } else if ak > self.k2 {
                if bk < self.k2 {
                    // |  <--|---
                    slice.elements.push(intersect::<I>(
                        &a,
                        &b,
                        self.k2,
                        calc_progress::<I>(&a, &b, self.k2),
                    ));
                    if bk < self.k1
                    // <--|-----|---
                    {
                        slice.elements.push(intersect::<I>(
                            &a,
                            &b,
                            self.k1,
                            calc_progress::<I>(&a, &b, self.k1),
                        ));
                    } else if i == len - 2 {
                        slice.elements.push(b); // last point
                    }
                }
            } else {
                // | --> |
                slice.elements.push(a);
                if bk < self.k1 {
                    // <--|---  |
                    slice.elements.push(intersect::<I>(
                        &a,
                        &b,
                        self.k1,
                        calc_progress::<I>(&a, &b, self.k1),
                    ));
                } else if bk > self.k2 {
                    // |  ---|-->
                    slice.elements.push(intersect::<I>(
                        &a,
                        &b,
                        self.k2,
                        calc_progress::<I>(&a, &b, self.k2),
                    ));
                }
            }
        }

        // close the polygon if its endpoints are not the same after clipping
        if !slice.elements.is_empty() {
            let first = slice.elements.first();
            let last = slice.elements.last();
            if first != last {
                slice.elements.push(*first.unwrap());
            }
        }

        slice
    }
}

/* clip features between two axis-parallel lines:
 *     |        |
 *  ___|___     |     /
 * /   |   \____|____/
 *     |        |
 */
pub fn clip<const I: usize>(
    features: &VtFeatures,
    k1: f64,
    k2: f64,
    min_all: f64,
    max_all: f64,
    line_metrics: bool,
) -> VtFeatures {
    // trivial accept
    if min_all >= k1 && max_all < k2 {
        return features.clone();
    }

    // trivial reject
    if max_all < k1 || min_all >= k2 {
        return VtFeatures::new();
    }

    let mut clipped: VtFeatures = Vec::with_capacity(features.len());

    for feature in features {
        let geom = &feature.geometry;
        let props = &feature.properties;
        let id = &feature.id;

        let min = GetCoordinate::<I>::get(&feature.bbox.min);
        let max = GetCoordinate::<I>::get(&feature.bbox.max);

        if min >= k1 && max < k2 {
            // trivial accept
            clipped.push(feature.clone());
        } else if max < k1 || min >= k2 {
            // trivial reject
            continue;
        } else {
            let clipped_geom = Clipper::<I>::new(k1, k2, line_metrics).clip_geometry(geom);

            match &clipped_geom {
                VtGeometry::MultiLineString(result) => {
                    if line_metrics {
                        for segment in result {
                            clipped.push(
                                VtFeature::new(
                                    VtGeometry::LineString(segment.clone()),
                                    props.clone(),
                                    id.clone(),
                                )
                                .unwrap(),
                            );
                        }
                    } else {
                        if let Some(feature) =
                            VtFeature::new(clipped_geom, props.clone(), id.clone())
                        {
                            clipped.push(feature);
                        }
                    }
                }
                _ => {
                    if let Some(feature) = VtFeature::new(clipped_geom, props.clone(), id.clone()) {
                        clipped.push(feature);
                    }
                }
            }
        }
    }

    clipped
}
