#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::io::BufReader;
    use geojson::GeoJson;
    use crate::{GeoJSONVT, Options};
    use crate::tile::EMPTY_TILE;
    use crate::types::VtPoint;

    #[test]
    fn us_states() {
        let geojson = geojson::GeoJson::from_reader(BufReader::new(File::open("fixtures/us-states.json").unwrap())).unwrap();
        let mut index = GeoJSONVT::new(&match geojson {
            GeoJson::Geometry(_) => unimplemented!(),
            GeoJson::Feature(_) => unimplemented!(),
            GeoJson::FeatureCollection(c) => c,
        }, &Options::default());

        let features = &index.get_tile(7, 37, 48).features;
        //let expected = parseJSONTile(loadFile("test/fixtures/us-states-z7-37-48.json"));
        //assert_eq!(features == expected, true);

        //let square = parseJSONTile(loadFile("test/fixtures/us-states-square.json"));
        let features = &index.get_tile(9, 148, 192).features;
        //assert_eq!(square == features, true); // clipped square

        assert_eq!(&EMPTY_TILE == index.get_tile(11, 800, 400), true);   // non-existing tile
        //assert_eq!(&EMPTY_TILE == index.get_tile(11, 800, 400), true); // non-existing tile

        // This test does not make sense in C++, since the parameters are cast to integers anyway.
        // assert_eq!(isEmpty(index.getTile(-5, 123.25, 400.25)), true); // invalid tile

        assert_eq!(37, index.total);
    }
}
