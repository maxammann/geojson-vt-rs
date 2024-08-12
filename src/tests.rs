use crate::clip::Clipper;
use crate::simplify::{simplify, simplify_wrapper};
use crate::tile::EMPTY_TILE;
use crate::types::{VtGeometry, VtLineString, VtMultiLineString, VtPoint};
use crate::{GeoJSONVT, Options};
use euclid::approxeq::ApproxEq;
use geojson::GeoJson;
use std::fs::File;
use std::io::BufReader;

fn ulps_eq(it: &f64, other: &f64, max_ulps: u32) -> bool {
    // Adapted from https://github.com/brendanzab/approx
    // Implementation based on: [Comparing Floating Point Numbers, 2012 Edition]
    // (https://randomascii.wordpress.com/2012/02/25/comparing-floating-point-numbers-2012-edition/)

    // Trivial negative sign check
    if it.signum() != other.signum() {
        // Handle -0 == +0
        return it == other;
    }

    let int_self: i64 = unsafe { std::mem::transmute(*it) };
    let int_other: i64 = unsafe { std::mem::transmute(*other) };

    i64::abs(int_self - int_other) < max_ulps as i64
}

fn multi_line_string_eq((a, b): (&VtMultiLineString, &VtMultiLineString)) -> bool {
    a.iter().zip(b.iter()).all(line_string_eq)
}

fn line_string_eq((a, b): (&VtLineString, &VtLineString)) -> bool {
    a.elements
        .iter()
        .zip(b.elements.iter())
        .all(|(a, b)| a.x == b.x && a.y == b.y)
}

macro_rules! parse_points {
    // Match a block containing tuples separated by commas
    ({ $($x:expr, $y:expr),* $(,)? }) => {
        vec![
            // Generate a VtPoint for each tuple
            $( VtPoint::new_without_z($x, $y), )*
        ]
    };
}

#[test]
fn simplify_points() {
    let mut points = vec![
        VtPoint::new_without_z(0.22455, 0.25015),
        VtPoint::new_without_z(0.22691, 0.24419),
        VtPoint::new_without_z(0.23331, 0.24145),
        VtPoint::new_without_z(0.23498, 0.23606),
        VtPoint::new_without_z(0.24421, 0.23276),
        VtPoint::new_without_z(0.26259, 0.21531),
        VtPoint::new_without_z(0.26776, 0.21381),
        VtPoint::new_without_z(0.27357, 0.20184),
        VtPoint::new_without_z(0.27312, 0.19216),
        VtPoint::new_without_z(0.27762, 0.18903),
        VtPoint::new_without_z(0.28036, 0.18141),
        VtPoint::new_without_z(0.28651, 0.17774),
        VtPoint::new_without_z(0.29241, 0.15937),
        VtPoint::new_without_z(0.29691, 0.15564),
        VtPoint::new_without_z(0.31495, 0.15137),
        VtPoint::new_without_z(0.31975, 0.14516),
        VtPoint::new_without_z(0.33033, 0.13757),
        VtPoint::new_without_z(0.34148, 0.13996),
        VtPoint::new_without_z(0.36998, 0.13789),
        VtPoint::new_without_z(0.38739, 0.14251),
        VtPoint::new_without_z(0.39128, 0.13939),
        VtPoint::new_without_z(0.40952, 0.14114),
        VtPoint::new_without_z(0.41482, 0.13975),
        VtPoint::new_without_z(0.42772, 0.12730),
        VtPoint::new_without_z(0.43960, 0.11974),
        VtPoint::new_without_z(0.47493, 0.10787),
        VtPoint::new_without_z(0.48651, 0.10675),
        VtPoint::new_without_z(0.48920, 0.10945),
        VtPoint::new_without_z(0.49379, 0.10863),
        VtPoint::new_without_z(0.50474, 0.11966),
        VtPoint::new_without_z(0.51296, 0.12235),
        VtPoint::new_without_z(0.51863, 0.12089),
        VtPoint::new_without_z(0.52409, 0.12688),
        VtPoint::new_without_z(0.52957, 0.12786),
        VtPoint::new_without_z(0.53421, 0.14093),
        VtPoint::new_without_z(0.53927, 0.14724),
        VtPoint::new_without_z(0.56769, 0.14891),
        VtPoint::new_without_z(0.57525, 0.15726),
        VtPoint::new_without_z(0.58062, 0.15815),
        VtPoint::new_without_z(0.60153, 0.15685),
        VtPoint::new_without_z(0.61774, 0.15986),
        VtPoint::new_without_z(0.62200, 0.16704),
        VtPoint::new_without_z(0.62955, 0.19460),
        VtPoint::new_without_z(0.63890, 0.19561),
        VtPoint::new_without_z(0.64126, 0.20081),
        VtPoint::new_without_z(0.65177, 0.20456),
        VtPoint::new_without_z(0.67155, 0.22255),
        VtPoint::new_without_z(0.68368, 0.21745),
        VtPoint::new_without_z(0.69525, 0.21915),
        VtPoint::new_without_z(0.70064, 0.21798),
        VtPoint::new_without_z(0.70312, 0.21436),
        VtPoint::new_without_z(0.71226, 0.21587),
        VtPoint::new_without_z(0.72149, 0.21281),
        VtPoint::new_without_z(0.72781, 0.21336),
        VtPoint::new_without_z(0.72998, 0.20873),
        VtPoint::new_without_z(0.73532, 0.20820),
        VtPoint::new_without_z(0.73994, 0.20477),
        VtPoint::new_without_z(0.76998, 0.20842),
        VtPoint::new_without_z(0.77960, 0.21687),
        VtPoint::new_without_z(0.78420, 0.21816),
        VtPoint::new_without_z(0.80024, 0.21462),
        VtPoint::new_without_z(0.81053, 0.21973),
        VtPoint::new_without_z(0.81719, 0.22682),
        VtPoint::new_without_z(0.82077, 0.23617),
        VtPoint::new_without_z(0.82723, 0.23616),
        VtPoint::new_without_z(0.82989, 0.23989),
        VtPoint::new_without_z(0.85100, 0.24894),
        VtPoint::new_without_z(0.85988, 0.25549),
        VtPoint::new_without_z(0.86521, 0.26853),
        VtPoint::new_without_z(0.85795, 0.28030),
        VtPoint::new_without_z(0.86548, 0.29145),
        VtPoint::new_without_z(0.86681, 0.29866),
        VtPoint::new_without_z(0.86468, 0.30271),
        VtPoint::new_without_z(0.86779, 0.30617),
        VtPoint::new_without_z(0.85987, 0.31137),
        VtPoint::new_without_z(0.86008, 0.31435),
        VtPoint::new_without_z(0.85829, 0.31494),
        VtPoint::new_without_z(0.85810, 0.32760),
        VtPoint::new_without_z(0.85454, 0.33540),
        VtPoint::new_without_z(0.86092, 0.34300),
        VtPoint::new_without_z(0.85643, 0.35015),
        VtPoint::new_without_z(0.85142, 0.35296),
        VtPoint::new_without_z(0.84984, 0.35959),
        VtPoint::new_without_z(0.85456, 0.36553),
        VtPoint::new_without_z(0.84974, 0.37038),
        VtPoint::new_without_z(0.84409, 0.37189),
        VtPoint::new_without_z(0.84475, 0.38044),
        VtPoint::new_without_z(0.84152, 0.38367),
        VtPoint::new_without_z(0.83957, 0.39040),
        VtPoint::new_without_z(0.84559, 0.39905),
        VtPoint::new_without_z(0.84840, 0.40755),
        VtPoint::new_without_z(0.84371, 0.41130),
        VtPoint::new_without_z(0.84409, 0.41988),
        VtPoint::new_without_z(0.83951, 0.43276),
        VtPoint::new_without_z(0.84133, 0.44104),
        VtPoint::new_without_z(0.84762, 0.44922),
        VtPoint::new_without_z(0.84716, 0.45844),
        VtPoint::new_without_z(0.85138, 0.46279),
        VtPoint::new_without_z(0.85397, 0.47115),
        VtPoint::new_without_z(0.86636, 0.48077),
    ];

    let simplified = vec![
        VtPoint::new_without_z(0.22455, 0.25015),
        VtPoint::new_without_z(0.26776, 0.21381),
        VtPoint::new_without_z(0.29691, 0.15564),
        VtPoint::new_without_z(0.33033, 0.13757),
        VtPoint::new_without_z(0.40952, 0.14114),
        VtPoint::new_without_z(0.4396, 0.11974),
        VtPoint::new_without_z(0.48651, 0.10675),
        VtPoint::new_without_z(0.52957, 0.12786),
        VtPoint::new_without_z(0.53927, 0.14724),
        VtPoint::new_without_z(0.56769, 0.14891),
        VtPoint::new_without_z(0.61774, 0.15986),
        VtPoint::new_without_z(0.62955, 0.1946),
        VtPoint::new_without_z(0.67155, 0.22255),
        VtPoint::new_without_z(0.72781, 0.21336),
        VtPoint::new_without_z(0.73994, 0.20477),
        VtPoint::new_without_z(0.76998, 0.20842),
        VtPoint::new_without_z(0.7842, 0.21816),
        VtPoint::new_without_z(0.80024, 0.21462),
        VtPoint::new_without_z(0.82077, 0.23617),
        VtPoint::new_without_z(0.85988, 0.25549),
        VtPoint::new_without_z(0.86521, 0.26853),
        VtPoint::new_without_z(0.85795, 0.2803),
        VtPoint::new_without_z(0.86779, 0.30617),
        VtPoint::new_without_z(0.85829, 0.31494),
        VtPoint::new_without_z(0.85454, 0.3354),
        VtPoint::new_without_z(0.86092, 0.343),
        VtPoint::new_without_z(0.84984, 0.35959),
        VtPoint::new_without_z(0.85456, 0.36553),
        VtPoint::new_without_z(0.84409, 0.37189),
        VtPoint::new_without_z(0.83957, 0.3904),
        VtPoint::new_without_z(0.8484, 0.40755),
        VtPoint::new_without_z(0.83951, 0.43276),
        VtPoint::new_without_z(0.85397, 0.47115),
        VtPoint::new_without_z(0.86636, 0.48077),
    ];

    simplify_wrapper(&mut points, 0.001);

    let mut result = Vec::new();
    for p in points {
        if p.z > 0.005 * 0.005 {
            result.push(p);
        }
    }

    assert_eq!(result.len(), simplified.len());
    assert!(result
        .iter()
        .zip(simplified.iter())
        .all(|(a, b)| ulps_eq(&a.x, &b.x, 4) && ulps_eq(&a.y, &b.y, 4)));
}

#[test]
fn us_states() {
    let geojson = geojson::GeoJson::from_reader(BufReader::new(
        File::open("fixtures/us-states.json").unwrap(),
    ))
    .unwrap();
    let mut index = GeoJSONVT::new(
        &match geojson {
            GeoJson::Geometry(_) => unimplemented!(),
            GeoJson::Feature(_) => unimplemented!(),
            GeoJson::FeatureCollection(c) => c,
        },
        &Options::default(),
    );

    let features = &index.get_tile(7, 37, 48).features;
    //let expected = parseJSONTile(loadFile("test/fixtures/us-states-z7-37-48.json"));
    //assert_eq!(features == expected, true);

    //let square = parseJSONTile(loadFile("test/fixtures/us-states-square.json"));
    let features = &index.get_tile(9, 148, 192).features;
    //assert_eq!(square == features, true); // clipped square

    assert_eq!(&EMPTY_TILE == index.get_tile(11, 800, 400), true); // non-existing tile
                                                                   //assert_eq!(&EMPTY_TILE == index.get_tile(11, 800, 400), true); // non-existing tile

    // This test does not make sense in C++, since the parameters are cast to integers anyway.
    // assert_eq!(isEmpty(index.getTile(-5, 123.25, 400.25)), true); // invalid tile

    assert_eq!(37, index.total);
}

#[test]
fn clip_polylines() {
    let points1 = VtLineString::from_slice(&[
        VtPoint::new_without_z(0., 0.),
        VtPoint::new_without_z(50., 0.),
        VtPoint::new_without_z(50., 10.),
        VtPoint::new_without_z(20., 10.),
        VtPoint::new_without_z(20., 20.),
        VtPoint::new_without_z(30., 20.),
        VtPoint::new_without_z(30., 30.),
        VtPoint::new_without_z(50., 30.),
        VtPoint::new_without_z(50., 40.),
        VtPoint::new_without_z(25., 40.),
        VtPoint::new_without_z(25., 50.),
        VtPoint::new_without_z(0., 50.),
        VtPoint::new_without_z(0., 60.),
        VtPoint::new_without_z(25., 60.),
        VtPoint::new_without_z(30., 60.),
    ]);

    let points2 = VtLineString::from_slice(&[
        VtPoint::new_without_z(0., 0.),
        VtPoint::new_without_z(50., 0.),
        VtPoint::new_without_z(50., 10.),
        VtPoint::new_without_z(0., 10.),
    ]);

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
        &match clipped1 {
            VtGeometry::MultiLineString(s) => s,
            _ => unreachable!(),
        }
    )));

    assert!(multi_line_string_eq((
        &expected2,
        &match clipped2 {
            VtGeometry::MultiLineString(s) => s,
            _ => unreachable!(),
        }
    )));
}
