# STL Format Notes

Binary STL is selected when its 80-byte header, little-endian triangle count, and exact `84 + 50 * count` file length agree. This safely handles binary headers that begin with `solid`. Otherwise, valid UTF-8 is parsed as strict ASCII STL with `solid`, `facet normal`, `outer loop`, exactly three `vertex` records, `endloop`, `endfacet`, and `endsolid`.

Both paths reject non-finite coordinates and report malformed structure through `Result`. File normals are retained but geometric checks calculate areas and shape from vertex positions. Duplicate vertices use exact `f32` values, with signed zero normalized; nearby coordinates are intentionally not welded.

Quality thresholds are configurable in `config.json`. A sliver has longest-edge-squared divided by twice its area at or above `sliver_aspect_ratio`. A large triangle consumes at least `large_triangle_area_ratio` of bounding-box diagonal squared. Incident-edge counts use unique, non-self indexed edges.
