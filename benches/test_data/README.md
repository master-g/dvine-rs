# KG Test Data Files

This directory contains real KG image files extracted from the game, used for benchmarking and testing.

## Files

### BLACK
- **Size**: 1.1 KB
- **Dimensions**: 640x480 pixels
- **Description**: Simple black image file
- **Use case**: Testing minimal compression scenarios, baseline performance
- **Compression ratio**: Very high (simple content compresses well)

### VYADOY01
- **Size**: 126 KB
- **Dimensions**: 640x480 pixels
- **Description**: Complex game scene with detailed graphics
- **Use case**: Real-world performance testing with typical game content
- **Compression ratio**: Lower (complex content doesn't compress as well)

## Purpose

These files serve multiple purposes:

1. **Realistic Benchmarking**: Unlike synthetic test data, these files represent actual game assets with real compression patterns
2. **Regression Testing**: Consistent test files ensure performance changes are detected
3. **Diverse Workloads**: BLACK tests best-case compression, VYADOY01 tests typical-case

## Source

These files are copied from `bin/kg_extract/` which contains all KG files extracted from the game archives.

## Usage in Benchmarks

```rust
// Files are automatically loaded in benchmarks
cargo bench --manifest-path benches/Cargo.toml kg_decompress_real
```

The benchmark suite will:
- Load these files from `benches/test_data/`
- Measure decompression performance
- Compare against synthetic test data
- Report throughput in pixels/second

## Adding More Test Files

To add additional test files for benchmarking:

1. Copy the file from `bin/kg_extract/`:
   ```bash
   cp bin/kg_extract/FILENAME benches/test_data/
   ```

2. Update `benches/benches/kg_decode.rs` to include the new file in the `test_files` vector

3. Document the file in this README

## File Format

All files follow the KG format specification:
- 32-byte header
- Optional padding
- 256-color palette (1024 bytes)
- BPP3 compressed image data

See `crates/dvine_types/src/file/kg/` for the decoder implementation.

## Characteristics Comparison

| File | Size | Complexity | Compression | Benchmark Focus |
|------|------|------------|-------------|-----------------|
| BLACK | 1.1 KB | Very Low | Excellent | Best-case, header parsing |
| VYADOY01 | 126 KB | High | Typical | Real-world, worst-case |

The size difference (116x) demonstrates how compression effectiveness varies with content complexity.

## Notes

- These are binary files, do not edit
- Files are committed to the repository for reproducible benchmarks
- Total size: ~127 KB (acceptable for version control)
- Files represent actual game data circa 1990s

---

**Last Updated**: 2024  
**Source**: dvine-rs game archives