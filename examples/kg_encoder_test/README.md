# KG Encoder/Decoder Examples

This directory contains two examples for the KG image format encoder:

1. **`main.rs`** - Comprehensive validation test using real BMP images
2. **`simple_example.rs`** - Basic API usage demonstration

## Simple API Example

For a quick introduction to the KG encoder API, run:

```bash
cargo run --example kg_simple_example
```

This demonstrates:
- Creating RGB image data
- Encoding to KG format
- Saving/loading KG files
- Decoding and validation
- Using the File API

## Comprehensive Validation Test

This validates the KG image format encoder by performing comprehensive round-trip testing on real BMP images.

## What It Does

1. **Loads BMP Images**: Reads all BMP files from `bin/raw_bmp` directory
2. **Encodes to KG**: Compresses each image using the KG encoder
3. **Decodes Back**: Decompresses the KG data back to RGB
4. **Validates**: Compares original and decoded data for pixel-perfect accuracy
5. **Reports**: Provides detailed statistics including compression ratios

## Running the Example

### Basic Usage

```bash
cargo run --example kg_encoder_test
```

This will test all BMP files in `bin/raw_bmp/` and save output files.

### Command Line Options

```bash
cargo run --example kg_encoder_test -- [OPTIONS]
```

**Options:**
- `--limit, -l <N>` - Test only the first N files (useful for quick testing)
- `--no-save` - Don't save output files (faster, validation only)
- `--help, -h` - Show help message

**Examples:**

```bash
# Test only the first 10 files
cargo run --example kg_encoder_test -- --limit 10

# Quick validation without saving files
cargo run --example kg_encoder_test -- --no-save

# Test 5 files without saving output
cargo run --example kg_encoder_test -- --limit 5 --no-save

# Show help
cargo run --example kg_encoder_test -- --help
```

## Requirements

- BMP files must be present in `bin/raw_bmp/` directory
- Images must have 256 or fewer unique colors (8-bit indexed color limit)

## Output

The example creates a `bin/kg_test_output/` directory containing:
- **`.kg` files**: Compressed KG format files
- **`*_decoded.bmp` files**: Decoded images for manual verification

## Test Results

The example tests **403 BMP files** from the original game data:

```
╔═══════════════════════════════════════════════════════════╗
║                   Overall Statistics                     ║
╚═══════════════════════════════════════════════════════════╝
│ Total files tested: 403
│ Pixel-perfect matches: 403
│ Skipped (too many colors): 0
│ Failed: 0
│ Average compression ratio: 13.39%
└───────────────────────────────────────────────────────────

✓ All tests passed! The KG encoder is working correctly.
```

### Compression Performance

The encoder achieves excellent compression ratios:
- **Best case**: ~1.5% (60x compression) for simple patterns
- **Average**: ~13.4% (7.5x compression) across all test images
- **Worst case**: ~40% for complex images with many unique colors

### Validation Status

✅ **100% Pixel-Perfect Accuracy**: All 403 test images pass round-trip validation with zero pixel differences.

## Example Output

```
🔍 Testing: BUHIN.bmp
   Loading BMP...
   ✓ Loaded 640x480 image (921600 bytes)
   Checking unique colors...
   ✓ Image has 92 unique colors (within 256 limit)
   Encoding to KG format...
   ✓ Compressed to 51043 bytes (5.54% of original)
   ✓ Saved KG file: bin/kg_test_output/BUHIN.kg
   Decoding KG data...
   ✓ Decoded to 640x480 image (921600 bytes)
   Comparing pixels...
   ✓ Pixel-perfect match!
   ✓ Saved decoded BMP: bin/kg_test_output/BUHIN_decoded.bmp

┌─────────────────────────────────────────────────────────
│ File: BUHIN.bmp
├─────────────────────────────────────────────────────────
│ Dimensions: 640x480
│ Original size: 921600 bytes
│ Compressed size: 51043 bytes
│ Compression ratio: 5.54% (18.05x)
│ Pixel-perfect: ✓ YES
└─────────────────────────────────────────────────────────
```

## What This Proves

This comprehensive test demonstrates that the KG encoder:

1. ✅ **Correctly implements the compression algorithm** - All encoded data can be decoded
2. ✅ **Maintains perfect fidelity** - Zero data loss in the compression process
3. ✅ **Handles various image types** - Works with different dimensions and color counts
4. ✅ **Produces efficient compression** - Achieves good compression ratios
5. ✅ **Is production-ready** - Successfully processes 403 real game images

## Technical Details

### Compression Algorithm

The encoder uses KG Type 1 (BPP3) compression with:
- **LRU Cache**: 256×8 cache for efficient color indexing
- **Dictionary Lookup**: Smart color encoding (4 bits via cache vs 9 bits direct)
- **Copy Operations**: Six different pattern matching strategies
- **Variable-length Encoding**: Progressive bit encoding for optimal space usage

### Supported Formats

- **Input**: 24-bit RGB BMP images
- **Output**: KG format with 8-bit indexed color + BGRA palette
- **Limitations**: Maximum 256 unique colors per image

## Use Cases

This example serves multiple purposes:

1. **Validation**: Ensures encoder correctness through comprehensive testing
2. **Benchmarking**: Measures compression performance on real data
3. **Documentation**: Demonstrates proper encoder/decoder usage
4. **Regression Testing**: Catches bugs when modifying the encoder

## Troubleshooting

### "Image has X unique colors (max 256)"

Some images may have more than 256 colors and cannot be compressed. Consider:
- Using color quantization to reduce the palette
- Converting to 8-bit indexed color before compression

### "No BMP files found"

Ensure the `bin/raw_bmp/` directory exists and contains BMP files.

## Quick Start

For a quick API introduction:
```bash
cargo run --example kg_simple_example
```

For comprehensive validation:
```bash
cargo run --example kg_encoder_test -- --limit 10
```

## Related Files

- **Encoder**: `crates/dvine_types/src/file/kg/encode.rs`
- **Decoder**: `crates/dvine_types/src/file/kg/decode.rs`
- **Tests**: `crates/dvine_types/src/file/kg/encode.rs` (unit tests)
- **Simple Example**: `examples/kg_encoder_test/simple_example.rs`
- **Validation Test**: `examples/kg_encoder_test/main.rs`