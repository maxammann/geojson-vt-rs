use std::collections::HashMap;
use std::f64::consts::PI;
use std::fs::File;
use std::io::{BufReader, Read};
use std::str::FromStr;
use approx::UlpsEq;

use euclid::approxeq::ApproxEq;
use geojson::{
    Feature, FeatureCollection, GeoJson, Geometry, JsonValue, LineStringType,
    PointType, PolygonType, Position,
};
use geojson::feature::Id;
use serde_json::{Number, Value};

use crate::{
    geojson_to_tile, GeoJSONVT, LinearRingType, MultiLineStringType, Options, TileOptions,
};
use crate::clip::Clipper;
use crate::simplify::simplify_wrapper;
use crate::tile::EMPTY_TILE;
use crate::types::*;

macro_rules! points {
    // Match a block containing tuples separated by commas
     ($({$x:expr, $y:expr}),*) => {
        [
            // Generate a VtPoint for each tuple
            $( VtPoint::new_without_z($x as f64, $y as f64), )*
        ]
    };
}

fn multi_line_string_eq((a, b): (&VtMultiLineString, &VtMultiLineString)) -> bool {
    a.iter().zip(b.iter()).all(line_string_eq)
}

fn line_string_eq((a, b): (&VtLineString, &VtLineString)) -> bool {
    points_eq((&a.elements, &b.elements))
}

fn linear_ring_eq((a, b): (&VtLinearRing, &VtLinearRing)) -> bool {
    points_eq((&a.elements, &b.elements))
}

fn points_eq((a, b): (&[VtPoint], &[VtPoint])) -> bool {
    a.iter()
        .zip(b.iter())
        .all(|(a, b)| (&a.x).ulps_eq(&b.x, 0.0, 4) && (&a.y).ulps_eq(&b.y, 0.0, 4))
}

fn polygon_eq((a, b): (&VtPolygon, &VtPolygon)) -> bool {
    a.iter().zip(b.iter()).all(linear_ring_eq)
}

fn parse_jsontiles(tiles: JsonValue) -> HashMap<String, FeatureCollection> {
    let Value::Object(tiles) = tiles else {
        panic!("not a valid tiles file");
    };

    tiles
        .into_iter()
        .map(|(key, value)| (key, parse_jsontile(value)))
        .collect()
}

fn parse_jsontile(tile: JsonValue) -> FeatureCollection {
    let mut features = Vec::new();
    assert!(matches!(tile, JsonValue::Array(_)));

    let JsonValue::Array(tile_features) = tile else {
        panic!("tile not an array")
    };

    for feature in tile_features {
        let mut feat = Feature {
            bbox: None,
            geometry: None,
            id: None,
            properties: None,
            foreign_members: None,
        };

        if let Some(JsonValue::Object(tile_feature)) = &feature.get("tags") {
            if tile_feature.is_empty() {
                feat.properties = None;
            } else {
                feat.properties = Some(tile_feature.clone());
            }
        }

        if let Some(JsonValue::String(tile_id)) = feature.get("id") {
            feat.id = Some(Id::String(tile_id.clone()))
        }

        if let Some(JsonValue::Number(tile_id)) = feature.get("id") {
            feat.id = Some(Id::Number(tile_id.clone()))
        }

        if let (Some(JsonValue::Number(tile_type)), Some(JsonValue::Array(tile_geom))) =
            (feature.get("type"), feature.get("geometry"))
        {
            let geom_type = tile_type.as_i64().unwrap();
            // point geometry
            if geom_type == 1 {
                if tile_geom.len() == 1 {
                    let pt = tile_geom.first().unwrap();
                    assert_eq!(pt.as_array().unwrap().len(), 2);
                    feat.geometry = Some(Geometry::new(geojson::Value::Point(PointType::from(&[
                        pt.get(0).unwrap().as_f64().unwrap(),
                        pt.get(1).unwrap().as_f64().unwrap(),
                    ]))))
                } else {
                    let mut points = vec![];
                    for pt in tile_geom {
                        points.push(PointType::from(&[
                            pt.get(0).unwrap().as_f64().unwrap(),
                            pt.get(1).unwrap().as_f64().unwrap(),
                        ]))
                    }
                    feat.geometry = Some(Geometry::new(geojson::Value::MultiPoint(points)))
                }
            } else if geom_type == 2 {
                // linestring geometry
                let mut multi_line: MultiLineStringType = Vec::new();
                let is_multi = tile_geom.len() > 1;
                for part in tile_geom {
                    let mut line_string: LineStringType = Vec::new();
                    for pt in part.as_array().unwrap() {
                        assert_eq!(pt.as_array().unwrap().len(), 2);
                        line_string.push(PointType::from(&[
                            pt.get(0).unwrap().as_f64().unwrap(),
                            pt.get(1).unwrap().as_f64().unwrap(),
                        ]));
                    }
                    if !is_multi {
                        feat.geometry =
                            Some(Geometry::new(geojson::Value::LineString(line_string)));
                        break;
                    } else {
                        multi_line.push(line_string);
                    }
                }

                if is_multi {
                    feat.geometry =
                        Some(Geometry::new(geojson::Value::MultiLineString(multi_line)));
                }
            } else if geom_type == 3 {
                // polygon geometry
                let mut poly: PolygonType = Vec::new();
                for ring in tile_geom {
                    let mut linear_ring: LinearRingType = Vec::new();
                    for pt in ring.as_array().unwrap() {
                        assert_eq!(pt.as_array().unwrap().len(), 2);
                        linear_ring.push(PointType::from(&[
                            pt.get(0).unwrap().as_f64().unwrap(),
                            pt.get(1).unwrap().as_f64().unwrap(),
                        ]));
                    }
                    poly.push(linear_ring);
                }
                feat.geometry = Some(Geometry::new(geojson::Value::Polygon(poly)));
            } else {
                panic!("unknown geometry type")
            }
        }

        features.push(feat);
    }

    return FeatureCollection {
        bbox: None,
        features,
        foreign_members: None,
    };
}

#[test]
fn simplify_points() {
    let mut points = Vec::from(points! {
        { 0.22455, 0.25015 }, { 0.22691, 0.24419 }, { 0.23331, 0.24145 }, { 0.23498, 0.23606 },
        { 0.24421, 0.23276 }, { 0.26259, 0.21531 }, { 0.26776, 0.21381 }, { 0.27357, 0.20184 },
        { 0.27312, 0.19216 }, { 0.27762, 0.18903 }, { 0.28036, 0.18141 }, { 0.28651, 0.17774 },
        { 0.29241, 0.15937 }, { 0.29691, 0.15564 }, { 0.31495, 0.15137 }, { 0.31975, 0.14516 },
        { 0.33033, 0.13757 }, { 0.34148, 0.13996 }, { 0.36998, 0.13789 }, { 0.38739, 0.14251 },
        { 0.39128, 0.13939 }, { 0.40952, 0.14114 }, { 0.41482, 0.13975 }, { 0.42772, 0.12730 },
        { 0.43960, 0.11974 }, { 0.47493, 0.10787 }, { 0.48651, 0.10675 }, { 0.48920, 0.10945 },
        { 0.49379, 0.10863 }, { 0.50474, 0.11966 }, { 0.51296, 0.12235 }, { 0.51863, 0.12089 },
        { 0.52409, 0.12688 }, { 0.52957, 0.12786 }, { 0.53421, 0.14093 }, { 0.53927, 0.14724 },
        { 0.56769, 0.14891 }, { 0.57525, 0.15726 }, { 0.58062, 0.15815 }, { 0.60153, 0.15685 },
        { 0.61774, 0.15986 }, { 0.62200, 0.16704 }, { 0.62955, 0.19460 }, { 0.63890, 0.19561 },
        { 0.64126, 0.20081 }, { 0.65177, 0.20456 }, { 0.67155, 0.22255 }, { 0.68368, 0.21745 },
        { 0.69525, 0.21915 }, { 0.70064, 0.21798 }, { 0.70312, 0.21436 }, { 0.71226, 0.21587 },
        { 0.72149, 0.21281 }, { 0.72781, 0.21336 }, { 0.72998, 0.20873 }, { 0.73532, 0.20820 },
        { 0.73994, 0.20477 }, { 0.76998, 0.20842 }, { 0.77960, 0.21687 }, { 0.78420, 0.21816 },
        { 0.80024, 0.21462 }, { 0.81053, 0.21973 }, { 0.81719, 0.22682 }, { 0.82077, 0.23617 },
        { 0.82723, 0.23616 }, { 0.82989, 0.23989 }, { 0.85100, 0.24894 }, { 0.85988, 0.25549 },
        { 0.86521, 0.26853 }, { 0.85795, 0.28030 }, { 0.86548, 0.29145 }, { 0.86681, 0.29866 },
        { 0.86468, 0.30271 }, { 0.86779, 0.30617 }, { 0.85987, 0.31137 }, { 0.86008, 0.31435 },
        { 0.85829, 0.31494 }, { 0.85810, 0.32760 }, { 0.85454, 0.33540 }, { 0.86092, 0.34300 },
        { 0.85643, 0.35015 }, { 0.85142, 0.35296 }, { 0.84984, 0.35959 }, { 0.85456, 0.36553 },
        { 0.84974, 0.37038 }, { 0.84409, 0.37189 }, { 0.84475, 0.38044 }, { 0.84152, 0.38367 },
        { 0.83957, 0.39040 }, { 0.84559, 0.39905 }, { 0.84840, 0.40755 }, { 0.84371, 0.41130 },
        { 0.84409, 0.41988 }, { 0.83951, 0.43276 }, { 0.84133, 0.44104 }, { 0.84762, 0.44922 },
        { 0.84716, 0.45844 }, { 0.85138, 0.46279 }, { 0.85397, 0.47115 }, { 0.86636, 0.48077 }
    });

    let simplified = Vec::from(points! {
        { 0.22455, 0.25015 }, { 0.26776, 0.21381 }, { 0.29691, 0.15564 }, { 0.33033, 0.13757 },
        { 0.40952, 0.14114 }, { 0.4396, 0.11974 },  { 0.48651, 0.10675 }, { 0.52957, 0.12786 },
        { 0.53927, 0.14724 }, { 0.56769, 0.14891 }, { 0.61774, 0.15986 }, { 0.62955, 0.1946 },
        { 0.67155, 0.22255 }, { 0.72781, 0.21336 }, { 0.73994, 0.20477 }, { 0.76998, 0.20842 },
        { 0.7842, 0.21816 },  { 0.80024, 0.21462 }, { 0.82077, 0.23617 }, { 0.85988, 0.25549 },
        { 0.86521, 0.26853 }, { 0.85795, 0.2803 },  { 0.86779, 0.30617 }, { 0.85829, 0.31494 },
        { 0.85454, 0.3354 },  { 0.86092, 0.343 },   { 0.84984, 0.35959 }, { 0.85456, 0.36553 },
        { 0.84409, 0.37189 }, { 0.83957, 0.3904 },  { 0.8484, 0.40755 },  { 0.83951, 0.43276 },
        { 0.85397, 0.47115 }, { 0.86636, 0.48077 }
    });

    simplify_wrapper(&mut points, 0.001);

    let mut result = Vec::new();
    for p in points {
        if p.z > 0.005 * 0.005 {
            result.push(p);
        }
    }

    assert_eq!(result.len(), simplified.len());
    assert!(points_eq((&result, &simplified)));
}

#[test]
fn clip_polylines() {
    let points1 = VtLineString::from_slice(&points! {
        { 0, 0 },   { 50, 0 },  { 50, 10 }, { 20, 10 },
        { 20, 20 }, { 30, 20 }, { 30, 30 }, { 50, 30 },
        { 50, 40 }, { 25, 40 }, { 25, 50 }, { 0, 50 },
        { 0, 60 },  { 25, 60 }, { 30, 60 }
    });

    let points2 = VtLineString::from_slice(&points! { { 0, 0 }, { 50, 0 }, { 50, 10 }, { 0, 10 } });

    let clip = Clipper::<0>::new(10., 40., false);

    let clipped1 = clip.clip_line_string(&points1);
    let clipped2 = clip.clip_line_string(&points2);

    let expected1 = VtMultiLineString::from(&[
        VtLineString::from_slice(&[
            VtPoint::new_without_z(10., 0.),
            VtPoint::new_without_z(40., 0.),
        ]),
        VtLineString::from_slice(&[
            VtPoint::new_without_z(40., 10.),
            VtPoint::new_without_z(20., 10.),
            VtPoint::new_without_z(20., 20.),
            VtPoint::new_without_z(30., 20.),
            VtPoint::new_without_z(30., 30.),
            VtPoint::new_without_z(40., 30.),
        ]),
        VtLineString::from_slice(&[
            VtPoint::new_without_z(40., 40.),
            VtPoint::new_without_z(25., 40.),
            VtPoint::new_without_z(25., 50.),
            VtPoint::new_without_z(10., 50.),
        ]),
        VtLineString::from_slice(&[
            VtPoint::new_without_z(10., 60.),
            VtPoint::new_without_z(25., 60.),
            VtPoint::new_without_z(30., 60.),
        ]),
    ]);

    let expected2 = VtMultiLineString::from(&[
        VtLineString::from_slice(&[
            VtPoint::new_without_z(10., 0.),
            VtPoint::new_without_z(40., 0.),
        ]),
        VtLineString::from_slice(&[
            VtPoint::new_without_z(40., 10.),
            VtPoint::new_without_z(10., 10.),
        ]),
    ]);

    assert!(multi_line_string_eq((
        &expected1,
        &clipped1.multi_line_string().unwrap()
    )));

    assert!(multi_line_string_eq((
        &expected2,
        &clipped2.multi_line_string().unwrap()
    )));
}

#[test]
fn clip_polylines_metric() {
    let points1 = VtLineString::from_slice(&points! {
        { 0, 0 },   { 50, 0 },  { 50, 10 }, { 20, 10 },
        { 20, 20 }, { 30, 20 }, { 30, 30 }, { 50, 30 },
        { 50, 40 }, { 25, 40 }, { 25, 50 }, { 0, 50 },
        { 0, 60 },  { 25, 60 }
    });

    let clip = Clipper::<0>::new(10., 40., true);

    let clipped = clip.clip_line_string(&points1).multi_line_string().unwrap();

    assert_eq!(clipped[0].seg_start, 10.0);
    assert_eq!(clipped[0].seg_end, 40.0);

    assert_eq!(clipped[1].seg_start, 70.0);
    assert_eq!(clipped[1].seg_end, 130.0);

    assert_eq!(clipped[2].seg_start, 160.0);
    assert_eq!(clipped[2].seg_end, 200.0);

    assert_eq!(clipped[3].seg_start, 230.0);
    assert_eq!(clipped[3].seg_end, 245.0);
}

#[test]
fn clip_polygons() {
    let points1 = VtPolygon::from(&[VtLinearRing::from_slice(&points! {
         { 0, 0 },
         { 50, 0 },
         { 50, 10 },
         { 20, 10 },
         { 20, 20 },
         { 30, 20 },
         { 30, 30 },
         { 50, 30 },
         { 50, 40 },
         { 25, 40 },
         { 25, 50 },
         { 0, 50 },
         { 0, 60 },
         { 25, 60 },
         { 0, 0 }
    })]);
    let points2 = VtPolygon::from(&[VtLinearRing::from_slice(&points! {
        { 0, 0 }, { 50, 0 }, { 50, 10 }, { 0, 10 }, { 0, 0 }
    })]);

    let clip = Clipper::<0>::new(10., 40., false);

    let clipped1 = clip.clip_polygon(&points1);
    let clipped2 = clip.clip_polygon(&points2);

    let expected1 = VtPolygon::from(&[VtLinearRing::from_slice(&points! {
        { 10, 0 },
        { 40, 0 },
        { 40, 10 },
        { 20, 10 },
        { 20, 20 },
        { 30, 20 },
        { 30, 30 },
        { 40, 30 },
        { 40, 40 },
        { 25, 40 },
        { 25, 50 },
        { 10, 50 },
        { 10, 60 },
        { 25, 60 },
        { 10, 24 },
        { 10, 0 }
    })]);

    let expected2 = VtPolygon::from(&[VtLinearRing::from_slice(&points! {
        { 10, 0 }, { 40, 0 }, { 40, 10 }, { 10, 10 }, { 10, 0 }
    })]);

    assert!(polygon_eq((&expected1, &clipped1.polygon().unwrap())));
    assert!(polygon_eq((&expected2, &clipped2.polygon().unwrap())));
}

#[test]
fn clip_points() {
    let points1 = VtMultiPoint::from(points! {
        { 0, 0 },   { 50, 0 },  { 50, 10 }, { 20, 10 },
        { 20, 20 }, { 30, 20 }, { 30, 30 }, { 50, 30 },
        { 50, 40 }, { 25, 40 }, { 25, 50 }, { 0, 50 },
        { 0, 60 },  { 25, 60 }
    });

    let points2 = VtMultiPoint::from(points! { { 0, 0 }, { 50, 0 }, { 50, 10 }, { 0, 10 } });

    let clip = Clipper::<0>::new(10., 40., false);

    let clipped1 = clip.clip_multi_point(&points1);
    let clipped2 = clip.clip_multi_point(&points2);

    let expected1 = VtMultiPoint::from(points! {
        { 20, 10 }, { 20, 20 }, { 30, 20 }, { 30, 30 }, { 25, 40 }, { 25, 50 }, { 25, 60 }
    });
    let expected2 = VtMultiPoint::from(points! {});

    assert!(points_eq((&expected1, &clipped1.multi_point().unwrap())));
    assert!(points_eq((&expected2, &clipped2.multi_point().unwrap())));
}

#[test]
fn get_tile_us_states() {
    let geojson = GeoJson::from_reader(BufReader::new(
        File::open("fixtures/us-states.json").unwrap(),
    ))
    .unwrap();
    let mut index = GeoJSONVT::from_geojson(&geojson, &Options::default());

    let features = &index.get_tile(7, 37, 48).features;
    let expected = parse_jsontile(
        serde_json::from_reader(File::open("fixtures/us-states-z7-37-48.json").unwrap()).unwrap(),
    );
    assert_eq!(features, &expected);

    let square = parse_jsontile(
        serde_json::from_reader(File::open("fixtures/us-states-square.json").unwrap()).unwrap(),
    );
    let features = &index.get_tile(9, 148, 192).features;
    assert_eq!(&square, features); // clipped square

    assert_eq!(&EMPTY_TILE == index.get_tile(11, 800, 400), true); // non-existing tile
    assert_eq!(&EMPTY_TILE == index.get_tile(11, 800, 400), true); // non-existing tile

    // This test does not make sense in C++/Rust, since the parameters are cast to integers anyway.
    // assert_eq!(isEmpty(index.getTile(-5, 123.25, 400.25)), true); // invalid tile

    assert_eq!(37, index.total);
}

#[test]
fn get_tile_generate_ids() {
    let geojson = GeoJson::from_reader(BufReader::new(
        File::open("fixtures/us-states.json").unwrap(),
    ))
    .unwrap();
    let mut index = GeoJSONVT::from_geojson(
        &geojson,
        &Options {
            max_zoom: 20,
            generate_id: true,
            tile: TileOptions {
                //tolerance: 0.0, disabling these make the test pass. Geometry is not checked in the C++ codebase, therefore the options work in the C++ version.
                //extent: 8192,
                ..TileOptions::default()
            },
            ..Options::default()
        },
    );

    let features = &index.get_tile(7, 37, 48).features;
    let expected = parse_jsontile(
        serde_json::from_reader(File::open("fixtures/us-states-z7-37-48-gen-ids.json").unwrap())
            .unwrap(),
    );
    assert_eq!(
        features.features.first().unwrap().id,
        Some(Id::Number(Number::from(6)))
    );
    assert_eq!(
        features.features.first().unwrap().id,
        Some(Id::Number(Number::from(6)))
    );
    assert_eq!(features, &expected);
}

#[test]
fn get_tile_antimerdian_triangle() {
    let geojson = GeoJson::from_reader(BufReader::new(
        File::open("fixtures/dateline-triangle.json").unwrap(),
    ))
    .unwrap();
    let mut index = GeoJSONVT::from_geojson(&geojson, &Options::default());

    #[derive(Copy, Clone, Debug)]
    struct TileCoordinate {
        z: u8,
        x: u32,
        y: u32,
    }

    let tile_coordinates = vec![
        TileCoordinate { z: 1, x: 0, y: 0 },
        TileCoordinate { z: 1, x: 0, y: 1 },
        TileCoordinate { z: 1, x: 1, y: 0 },
        TileCoordinate { z: 1, x: 1, y: 1 },
    ];

    for tile_coordinate in tile_coordinates {
        let tile = index.get_tile(tile_coordinate.z, tile_coordinate.x, tile_coordinate.y);
        assert_eq!(tile.num_points, tile.num_simplified);
        assert_eq!(
            tile.features.features.len(),
            1,
            "{tile_coordinate:?} is missing the feature"
        );
    }
}

#[test]
fn get_tile_polygon_clipping_bug() {
    let geojson = GeoJson::from_reader(BufReader::new(
        File::open("fixtures/polygon-bug.json").unwrap(),
    ))
    .unwrap();

    let mut index = GeoJSONVT::from_geojson(
        &geojson,
        &Options {
            tile: TileOptions {
                buffer: 1024,
                ..TileOptions::default()
            },
            ..Options::default()
        },
    );

    let tile = index.get_tile(5, 19, 9);
    assert_eq!(tile.features.features.len(), 1);
    assert_eq!(tile.num_points, 5);

    let expected = Geometry::new(geojson::Value::Polygon(PolygonType::from(&[vec![
        PointType::from(&[3072., 3072.]),
        PointType::from(&[5120., 3072.]),
        PointType::from(&[5120., 5120.]),
        PointType::from(&[3072., 5120.]),
        PointType::from(&[3072., 3072.]),
    ]])));

    let actual = tile.features.features[0].geometry.as_ref().unwrap();

    assert_eq!(actual, &expected);
}

#[test]
fn get_tile_projection() {
    let geojson = GeoJson::from_reader(BufReader::new(
        File::open("fixtures/linestring.json").unwrap(),
    ))
    .unwrap();

    let mut index = GeoJSONVT::from_geojson(
        &geojson,
        &Options {
            max_zoom: 20,
            tile: TileOptions {
                extent: 8192,
                tolerance: 0.,
                ..TileOptions::default()
            },
            ..Options::default()
        },
    );

    #[derive(Copy, Clone, Debug)]
    struct TileCoordinate {
        z: u8,
        x: u32,
        y: u32,
    }

    let tile_coordinates = vec![
        TileCoordinate { z: 0, x: 0, y: 0 },
        TileCoordinate { z: 1, x: 0, y: 0 },
        TileCoordinate { z: 2, x: 0, y: 1 },
        TileCoordinate { z: 3, x: 1, y: 3 },
        TileCoordinate { z: 4, x: 2, y: 6 },
        TileCoordinate { z: 5, x: 5, y: 12 },
        TileCoordinate { z: 6, x: 10, y: 24 },
        TileCoordinate { z: 7, x: 20, y: 49 },
        TileCoordinate { z: 8, x: 40, y: 98 },
        TileCoordinate {
            z: 9,
            x: 81,
            y: 197,
        },
        TileCoordinate {
            z: 10,
            x: 163,
            y: 395,
        },
        TileCoordinate {
            z: 11,
            x: 327,
            y: 791,
        },
        TileCoordinate {
            z: 12,
            x: 655,
            y: 1583,
        },
        TileCoordinate {
            z: 13,
            x: 1310,
            y: 3166,
        },
        TileCoordinate {
            z: 14,
            x: 2620,
            y: 6332,
        },
        TileCoordinate {
            z: 15,
            x: 5241,
            y: 12664,
        },
        TileCoordinate {
            z: 16,
            x: 10482,
            y: 25329,
        },
        TileCoordinate {
            z: 17,
            x: 20964,
            y: 50660,
        },
        TileCoordinate {
            z: 18,
            x: 41929,
            y: 101320,
        },
        TileCoordinate {
            z: 19,
            x: 83859,
            y: 202640,
        },
        TileCoordinate {
            z: 20,
            x: 167719,
            y: 405281,
        },
    ];

    for tile_coordinate in tile_coordinates {
        let tile = index.get_tile(tile_coordinate.z, tile_coordinate.x, tile_coordinate.y);
        assert_eq!(tile.num_points, tile.num_simplified);
        assert_eq!(tile.features.features.len(), 1);
        let geometry = &tile
            .features
            .features
            .first()
            .unwrap()
            .geometry
            .as_ref()
            .unwrap()
            .value;

        let line_string = match geometry {
            geojson::Value::LineString(line_string) => line_string,
            _ => panic!("not a linestring"),
        };
        assert_eq!(line_string.len(), 2);

        let total_features = (1u32 << tile_coordinate.z) as f64 * 8192.0;

        let to_web_mercator_lon = |point: &Position| {
            let x0 = 8192.0 * tile_coordinate.x as f64;
            return (x0 + point[0]) * 360.0 / total_features - 180.0;
        };

        let to_web_mercator_lat = |point: &Position| {
            let y0 = 8192.0 * tile_coordinate.y as f64;
            let y2 = 180.0 - (y0 + point[1]) * 360.0 / total_features;
            return 360.0 / PI * (y2 * PI / 180.0).exp().atan() - 90.0;
        };

        let tolerance = 0.1 / (1. + tile_coordinate.z as f64);

        assert!((-122.41822421550751f64)
            .approx_eq_eps(&to_web_mercator_lon(&line_string[0]), &tolerance));
        assert!(
            37.77852514599172f64.approx_eq_eps(&to_web_mercator_lat(&line_string[0]), &tolerance)
        );

        assert!((-122.41707086563109f64)
            .approx_eq_eps(&to_web_mercator_lon(&line_string[1]), &tolerance));
        assert!(
            37.780424620898664f64.approx_eq_eps(&to_web_mercator_lat(&line_string[1]), &tolerance)
        );
    }
}

fn gen_tiles(
    data: &str,
    max_zoom: u8,
    max_points: u32,
    line_metrics: bool,
) -> HashMap<String, FeatureCollection> {
    let geojson = GeoJson::from_str(data).unwrap();
    let mut index = GeoJSONVT::from_geojson(
        &geojson,
        &Options {
            max_zoom: 14,
            index_max_points: max_points,
            index_max_zoom: max_zoom,
            tile: TileOptions {
                line_metrics,
                ..TileOptions::default()
            },
            ..Options::default()
        },
    );

    let mut output = HashMap::new();

    let internal_tiles = index.get_internal_tiles().clone();
    for (_key, tile) in internal_tiles {
        let key = format!("z{}-{}-{}", tile.z, tile.x, tile.y);
        output.insert(key, index.get_tile(tile.z, tile.x, tile.y).features.clone());
    }

    output
}

struct Arguments {
    input_file: String,
    expected_file: String,
    max_zoom: u8,
    max_points: u32,
    line_metrics: bool,
}

impl Arguments {
    fn new(
        input_file: &str,
        expected_file: &str,
        max_zoom: u8,
        max_points: u32,
        line_metrics: bool,
    ) -> Self {
        Arguments {
            input_file: input_file.to_string(),
            expected_file: expected_file.to_string(),
            max_zoom,
            max_points,
            line_metrics,
        }
    }
}

#[test]
fn tile_tests() {
    let tests = [
        Arguments::new(
            "fixtures/us-states.json",
            "fixtures/us-states-tiles.json",
            7,
            200,
            false,
        ),
        Arguments::new(
            "fixtures/dateline.json",
            "fixtures/dateline-tiles.json",
            7,
            200,
            false,
        ),
        Arguments::new(
            "fixtures/dateline.json",
            "fixtures/dateline-metrics-tiles.json",
            0,
            10000,
            true,
        ),
        Arguments::new(
            "fixtures/feature.json",
            "fixtures/feature-tiles.json",
            0,
            10000,
            false,
        ),
        // In comparison to the C++ code we fixed the loading of MultiPoints in the parse_jsontile helper. Therefore, the below arguments succeed even when comparing the geometries.
        Arguments::new(
            "fixtures/collection.json",
            "fixtures/collection-tiles.json",
            0,
            10000,
            false,
        ),
        Arguments::new(
            "fixtures/single-geom.json",
            "fixtures/single-geom-tiles.json",
            0,
            10000,
            false,
        ),
    ];

    for test in tests {
        let mut file = File::open(&test.input_file).unwrap();
        let mut data = String::new();
        file.read_to_string(&mut data).unwrap();
        let mut actual = gen_tiles(&data, test.max_zoom, test.max_points, test.line_metrics);
        let expected = parse_jsontiles(
            serde_json::from_reader(File::open(&test.expected_file).unwrap()).unwrap(),
        );

        for (_key, value) in &mut actual {
            // The JSON files from the JS project flatten all MultiPolygon to a single Polygon with more rings. Do that here so we can compare the geometries.
            value.features = value
                .features
                .iter()
                .map(|feature| Feature {
                    bbox: feature.bbox.clone(),
                    geometry: feature.geometry.clone().map(|geom| {
                        Geometry::new(match geom.value {
                            geojson::Value::MultiPolygon(multi) => geojson::Value::Polygon(
                                multi.iter().flatten().cloned().collect::<Vec<_>>(),
                            ),
                            v => v,
                        })
                    }),
                    id: feature.id.clone(),
                    properties: feature.properties.clone(),
                    foreign_members: feature.foreign_members.clone(),
                })
                .collect();

            //fs::write(format!("us/actual/{}.json", _key), value.clone().to_string()).unwrap();
        }

        //for (key, value) in &expected {
        //    fs::write(format!("us/expected/{}.json", key), value.clone().to_string()).unwrap();
        //}

        assert_eq!(expected, actual);
    }
}

#[test]
fn geojson_to_tile_simple() {
    let geojson = GeoJson::from_reader(BufReader::new(
        File::open("fixtures/single-tile.json").unwrap(),
    ))
    .unwrap();

    let tile = geojson_to_tile(
        &geojson,
        12,
        1171,
        1566,
        &TileOptions::default(),
        false,
        false,
    );

    assert_eq!(tile.features.features.len(), 1);
    let props = tile
        .features
        .features
        .get(0)
        .as_ref()
        .unwrap()
        .properties
        .as_ref()
        .unwrap();
    let name = props.get("name").unwrap();
    let str = name.as_str().unwrap();
    assert_eq!(str, "P Street Northwest - Massachusetts Avenue Northwest");
}

#[test]
fn geojson_to_tile_clips() {
    let geojson = GeoJson::from_reader(BufReader::new(
        File::open("fixtures/us-states.json").unwrap(),
    ))
    .unwrap();

    let tile = geojson_to_tile(
        &geojson,
        12,
        1171,
        1566,
        &TileOptions::default(),
        false,
        true,
    );

    assert_eq!(tile.features.features.len(), 2);
    let props = tile
        .features
        .features
        .get(0)
        .as_ref()
        .unwrap()
        .properties
        .as_ref()
        .unwrap();
    let name = props.get("name").unwrap();
    let str = name.as_str().unwrap();
    assert_eq!(str, "District of Columbia");
}

#[test]
fn geojson_to_tile_metrics() {
    let geojson = GeoJson::from_reader(BufReader::new(
        File::open("fixtures/single-tile.json").unwrap(),
    ))
    .unwrap();

    let options = TileOptions {
        buffer: 64,
        tolerance: 3.,
        line_metrics: true,
        ..TileOptions::default()
    };

    let k_epsilon = 1e-5;

    let tile_left = geojson_to_tile(&geojson, 13, 2342, 3133, &options, false, false);
    assert_eq!(tile_left.features.features.len(), 1);

    let tile_right = geojson_to_tile(&geojson, 13, 2343, 3133, &options, false, false);
    assert_eq!(tile_right.features.features.len(), 1);

    let left_props = tile_left
        .features
        .features
        .get(0)
        .as_ref()
        .unwrap()
        .properties
        .as_ref()
        .unwrap();
    let left_clip_start = left_props
        .get("mapbox_clip_start")
        .unwrap()
        .as_f64()
        .unwrap();
    assert!((&left_clip_start).ulps_eq(&0.0, 0.0, 4));
    let left_clip_end = left_props.get("mapbox_clip_end").unwrap().as_f64().unwrap();
    assert!(left_clip_end.approx_eq_eps(&0.42103, &k_epsilon));

    let right_props = tile_right
        .features
        .features
        .get(0)
        .as_ref()
        .unwrap()
        .properties
        .as_ref()
        .unwrap();
    let right_clip_start = right_props
        .get("mapbox_clip_start")
        .unwrap()
        .as_f64()
        .unwrap();
    assert!(right_clip_start.approx_eq_eps(&0.40349, &k_epsilon));
    let right_clip_end = right_props
        .get("mapbox_clip_end")
        .unwrap()
        .as_f64()
        .unwrap();
    assert!((&right_clip_end).ulps_eq(&1.0, 0.0, 4)); // TODO verify this usage!
}

#[test]
fn geojson_to_tile_clip_vertex_on_tile_border() {
    let data = r#"{
        "type": "Feature",
        "geometry": {
            "type": "LineString",
            "coordinates":[
                [-77.031373697916663,38.895516493055553],
                [-77.01416015625,38.887532552083336],
                [-76.99,38.87]
            ]
        }
    }"#; // The second node is exactly on the (13, 2344, 3134) tile border.

    let geojson = GeoJson::from_str(data).unwrap();

    let options = Options {
        tile: TileOptions {
            extent: 8192,
            buffer: 2048,
            line_metrics: true,
            ..TileOptions::default()
        },
        ..Options::default()
    };

    let k_epsilon = 1e-5;

    let mut index = GeoJSONVT::from_geojson(&geojson, &options);

    let tile = index.get_tile(13, 2344, 3134);
    assert!(!tile.features.features.is_empty());

    let expected: LineStringType = LineStringType::from(&[vec![-2048., 2747.], vec![408., 5037.]]);

    let actual = match &tile.features.features[0].geometry.as_ref().unwrap().value {
        geojson::Value::LineString(line_string) => line_string,
        _ => panic!("must be linestring"),
    };
    assert_eq!(actual, &expected);

    // Check line metrics
    let props = tile
        .features
        .features
        .get(0)
        .as_ref()
        .unwrap()
        .properties
        .as_ref()
        .unwrap();
    let clip_start1 = props.get("mapbox_clip_start").unwrap().as_f64().unwrap();
    let clip_end1 = props.get("mapbox_clip_end").unwrap().as_f64().unwrap();
    assert!(0.660622.approx_eq_eps(&clip_start1, &k_epsilon));
    assert!(1.0.approx_eq_eps(&clip_end1, &k_epsilon));
}

/// See https://github.com/mapbox/geojson-vt/pull/176
/// And https://github.com/mapbox/geojson-vt/pull/173
#[test]
fn test_midpoint_calculation() {
    let geojson = GeoJson::from_reader(BufReader::new(
        File::open("fixtures/last_feature.json").unwrap(),
    ))
    .unwrap();
    let mut index = GeoJSONVT::from_geojson(
        &geojson,
        &Options {
            tile: TileOptions {
                tolerance: 3.0,
                ..TileOptions::default()
            },
            ..Options::default()
        },
    );
    let features = &index.get_tile(6, 11, 23).features;

    let expected1 = parse_jsontile(
        serde_json::from_reader(File::open("fixtures/last_feature-tile-fixed.json").unwrap())
            .unwrap(),
    );
    assert_ne!(features, &expected1);

    let expected2 = parse_jsontile(
        serde_json::from_reader(File::open("fixtures/last_feature-tile-broken-new.json").unwrap())
            .unwrap(),
    );
    assert_eq!(features, &expected2);
}
