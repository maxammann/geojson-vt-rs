use criterion::{criterion_group, criterion_main, Criterion};
use geojson_vt_rs::{geojson_to_tile, GeoJSONVT, Options, TileOptions};
use std::fs;
use std::hint::black_box;
use std::str::FromStr;

fn parse_geo_json(c: &mut Criterion) {
    let json = fs::read_to_string("data/countries.geojson").unwrap();
    c.bench_function("ParseGeoJSON", |b| {
        b.iter(|| geojson::GeoJson::from_str(&json).unwrap())
    });
}

fn generate_tile_index(c: &mut Criterion) {
    let json = fs::read_to_string("data/countries.geojson").unwrap();
    let features = geojson::GeoJson::from_str(&json).unwrap();
    let options = Options {
        index_max_zoom: 7,
        index_max_points: 200,
        tile: TileOptions {
            ..TileOptions::default()
        },
        ..Options::default()
    };

    c.bench_function("GenerateTileIndex", |b| {
        b.iter(|| {
            let index = GeoJSONVT::from_geojson(&features, &options);
            black_box(index);
        })
    });
}

fn traverse_tile_pyramid(c: &mut Criterion) {
    let json = fs::read_to_string("data/countries.geojson").unwrap();
    let features = geojson::GeoJson::from_str(&json).unwrap();
    let options = Options {
        index_max_zoom: 7,
        index_max_points: 200,
        tile: TileOptions {
            ..TileOptions::default()
        },
        ..Options::default()
    };

    let mut index = GeoJSONVT::from_geojson(&features, &options);

    c.bench_function("TraverseTilePyramid", |b| {
        b.iter(|| {
            let max_z = 11u8;
            for z in 0..max_z {
                let num_tiles = 2u32.pow(z as u32);
                for x in 0..num_tiles {
                    for y in 0..num_tiles {
                        index.get_tile(z, x, y);
                    }
                }
            }
        })
    });
}

fn large_geo_json_parse(c: &mut Criterion) {
    let json = fs::read_to_string("fixtures/points.geojson").unwrap();
    c.bench_function("LargeGeoJSONParse", |b| {
        b.iter(|| {
            geojson::GeoJson::from_str(&json).unwrap();
        })
    });
}

fn large_geo_json_tile_index(c: &mut Criterion) {
    let json = fs::read_to_string("fixtures/points.geojson").unwrap();
    let features = geojson::GeoJson::from_str(&json).unwrap();
    let options = Options::default();

    c.bench_function("LargeGeoJSONTileIndex", |b| {
        b.iter(|| {
            let index = GeoJSONVT::from_geojson(&features, &options);
            black_box(index)
        })
    });
}

fn large_geo_json_get_tile(c: &mut Criterion) {
    let json = fs::read_to_string("fixtures/points.geojson").unwrap();
    let features = geojson::GeoJson::from_str(&json).unwrap();
    let options = Options::default();

    let mut index = GeoJSONVT::from_geojson(&features, &options);
    c.bench_function("LargeGeoJSONGetTile", |b| {
        b.iter(|| {
            index.get_tile(12, 1171, 1566);
        })
    });
}
fn large_geo_json_to_tile(c: &mut Criterion) {
    let json = fs::read_to_string("data/countries.geojson").unwrap();
    let features = geojson::GeoJson::from_str(&json).unwrap();
    c.bench_function("LargeGeoJSONToTile", |b| {
        b.iter(|| {
            geojson_to_tile(
                &features,
                12,
                1171,
                1566,
                &TileOptions::default(),
                false,
                true,
            );
        })
    });
}
fn single_tile_index(c: &mut Criterion) {
    let json = fs::read_to_string("fixtures/single-tile.json").unwrap();
    let features = geojson::GeoJson::from_str(&json).unwrap();

    let options = Options {
        index_max_zoom: 7,
        index_max_points: 10000,
        tile: TileOptions {
            ..TileOptions::default()
        },
        ..Options::default()
    };

    let mut index = GeoJSONVT::from_geojson(&features, &options);

    c.bench_function("SingleTileIndex", |b| {
        b.iter(|| {
            index.get_tile(12, 1171, 1566);
        })
    });
}
fn single_tile_geo_json_to_tile(c: &mut Criterion) {
    let json = fs::read_to_string("fixtures/single-tile.json").unwrap();
    let features = geojson::GeoJson::from_str(&json).unwrap();
    c.bench_function("SingleTileGeoJSONToTile", |b| {
        b.iter(|| {
            geojson_to_tile(
                &features,
                12,
                1171,
                1566,
                &TileOptions::default(),
                false,
                true,
            );
        })
    });
}

criterion_group!(
    benches,
    parse_geo_json,
    generate_tile_index,
    traverse_tile_pyramid,
    large_geo_json_parse,
    large_geo_json_tile_index,
    large_geo_json_get_tile,
    large_geo_json_to_tile,
    single_tile_index,
    single_tile_geo_json_to_tile
);
criterion_main!(benches);
