# dvine-rs Benchmarks

Comprehensive performance benchmarking suite for KG file decoding and other dvine-rs components.

## Quick Start

```bash
# Run all benchmarks with HTML report
./benches/run_benchmarks.sh --kg --report

# Run only real file benchmarks
cargo bench --manifest-path benches/Cargo.toml kg_decompress_real

# Generate flamegraph for hot path analysis
./benches/run_benchmarks.sh --flamegraph --kg
```

## Overview

This benchmark suite provides:
- **Real file benchmarks** using actual game assets (BLACK, VYADOY01)
- **Synthetic benchmarks** for scalability testing (64x64 to 1920x1080)
- **Component benchmarks** for individual operations (header, palette, LRU cache, etc.)
- **Profiling support** with flamegraph generation
- **CI/CD integration** examples

**Current Status**: ✅ Phase 1 Complete (Criterion 0.7.0, Real files integrated)

## Benchmark Suites

### Real File Benchmarks (`kg_decompress_real`)

Tests decompression with actual game files:

| File | Size | Complexity | Performance | Use Case |
|------|------|------------|-------------|----------|
| BLACK | 1.1 KB | Very low | ~470 Mpixels/s | Best-case scenario |
| VYADOY01 | 126 KB | High | ~110 Mpixels/s | Real-world performance |

```bash
cargo bench --manifest-path benches/Cargo.toml kg_decompress_real
```

**Key Insight**: Content complexity causes 4.3x performance difference!

### Synthetic Benchmarks (`kg_decompress_synthetic`)

Tests various image sizes with generated data:
- 64x64 (tiny) - Quick validation
- 256x256 (small) - Small assets
- 512x512 (medium) - Medium assets
- 1024x768 (large) - Typical game assets
- 1920x1080 (xlarge) - HD resolution stress test

```bash
cargo bench --manifest-path benches/Cargo.toml kg_decompress_synthetic
```

### Component Benchmarks

| Benchmark | What It Tests | Command |
|-----------|---------------|---------|
| `kg_header` | Header parsing (~4.5ns) | `cargo bench kg_header` |
| `kg_palette` | Palette load & apply | `cargo bench kg_palette` |
| `kg_lru_cache` | Cache update performance | `cargo bench kg_lru_cache` |
| `kg_bit_ops` | Bit reading operations | `cargo bench kg_bit_ops` |
| `kg_copy_ops` | Memory copy patterns | `cargo bench kg_copy_ops` |
| `kg_realistic` | Full pipeline test | `cargo bench kg_realistic` |

All commands use: `--manifest-path benches/Cargo.toml`

## Key Findings

### Performance Characteristics

**Real File Results** (640x480 images):
- **BLACK**: 652µs (simple content, excellent compression)
- **VYADOY01**: 2.8ms (complex content, typical compression)

**Throughput**:
- Best case: 470 million pixels/second
- Typical case: 110 million pixels/second

### LRU Cache Optimization Discovery

Surprising result from micro-benchmarks:

| Implementation | Time per 1000 updates |
|----------------|----------------------|
| Manual loops | 1.13µs ⚡ |
| `copy_within` + `position` | 2.08µs |

**Conclusion**: For small arrays (8 elements), manual implementation is **45% faster** due to lower function call overhead.

### Expected Hot Paths

Based on code analysis (verify with flamegraph):

1. **Bit reading** (`read_bits`) - 30-40% of time
2. **Opcode processing** - 20-30% of time  
3. **LRU cache updates** - 10-15% of time
4. **Memory copies** - 10-15% of time
5. **Palette application** - 5-10% of time

## Finding Hot Paths

### Using Flamegraph

```bash
# Generate flamegraph
./benches/run_benchmarks.sh --flamegraph --kg

# Open and analyze
open flamegraph_kg.svg
```

**Reading the Flamegraph**:
- Wider bars = More CPU time
- Look for widest bars at the bottom
- Click to zoom into specific functions

### Using Criterion Reports

```bash
# Run benchmarks
cargo bench --manifest-path benches/Cargo.toml

# Open HTML report (macOS)
open target/criterion/report/index.html

# Open HTML report (Linux)
xdg-open target/criterion/report/index.html

# Or manually browse to: target/criterion/report/index.html
```

Reports include:
- Violin plots of timing distributions
- Trend analysis over multiple runs
- Statistical significance testing
- Throughput measurements

### Using System Profilers

**macOS (with Xcode installed):**
```bash
# Using Instruments (requires full Xcode, not just Command Line Tools)
cargo bench --manifest-path benches/Cargo.toml --no-run
instruments -t 'Time Profiler' target/release/deps/kg_decode-*
```

**macOS (Command Line Tools only):**
```bash
# Use flamegraph instead (cross-platform)
cargo install flamegraph
./benches/run_benchmarks.sh --flamegraph --kg
```

**Linux:**
```bash
# Using perf
cargo bench --manifest-path benches/Cargo.toml --no-run
perf record --call-graph dwarf target/release/deps/kg_decode-*
perf report

# Or use flamegraph
cargo install flamegraph
sudo cargo flamegraph --bench kg_decode --manifest-path benches/Cargo.toml
```

**Note**: If you see "xctrace requires Xcode" on macOS, install full Xcode from the App Store, or use flamegraph as an alternative.

## Optimization Workflow

### 1. Establish Baseline

```bash
./benches/run_benchmarks.sh --kg --baseline before
```

### 2. Make Changes

Edit your code, focusing on hot paths identified by profiling.

### 3. Compare Performance

```bash
./benches/run_benchmarks.sh --kg --compare before --report
```

### 4. Interpret Results

```
change: [-15.234% -14.891% -14.542%] (p = 0.00 < 0.05)
        Performance has improved.
```

- Negative % = Faster (improvement) ✅
- Positive % = Slower (regression) ❌
- p < 0.05 = Statistically significant

### 5. Iterate

Repeat until target performance is achieved.

## Test Data

### Real Files (`benches/test_data/`)

- **BLACK**: Minimal test file (1.1 KB)
  - 640x480 black image
  - Tests best-case compression
  - Fast baseline validation

- **VYADOY01**: Complex game scene (126 KB)
  - 640x480 detailed graphics
  - Tests real-world performance
  - Representative of actual game assets

See `test_data/README.md` for detailed file information.

### Synthetic Data

Generated on-the-fly by `benches/src/lib.rs`:
- Consistent test data
- Customizable dimensions
- Includes all opcode types
- Deterministic (reproducible)

## Running Benchmarks

### Basic Usage

```bash
# All benchmarks
cargo bench --manifest-path benches/Cargo.toml

# Specific group
cargo bench --manifest-path benches/Cargo.toml kg_decompress_real

# Quick test (fewer samples)
cargo bench --manifest-path benches/Cargo.toml -- --sample-size 10
```

### Using the Helper Script

```bash
./benches/run_benchmarks.sh [OPTIONS]

Options:
  --all           Run all benchmarks
  --kg            Run KG benchmarks only
  --flamegraph    Generate flamegraph
  --baseline NAME Save results as baseline
  --compare NAME  Compare against baseline
  --report        Open HTML report
  --help          Show help
```

### Examples

```bash
# Quick validation
./benches/run_benchmarks.sh --kg --report

# Before/after comparison
./benches/run_benchmarks.sh --kg --baseline v1
# ... make changes ...
./benches/run_benchmarks.sh --kg --compare v1

# Profiling
./benches/run_benchmarks.sh --flamegraph --kg
```

## CI/CD Integration

See `.github-workflow-example.yml` for a complete GitHub Actions setup that:
- Runs benchmarks on PRs
- Compares against base branch
- Detects performance regressions
- Generates flamegraphs
- Posts results as PR comments

## Adding New Benchmarks

### 1. Add Test Function

```rust
fn bench_new_feature(c: &mut Criterion) {
    let mut group = c.benchmark_group("my_feature");
    
    group.bench_function("operation", |b| {
        b.iter(|| {
            // Code to benchmark
        });
    });
    
    group.finish();
}
```

### 2. Register in criterion_group!

```rust
criterion_group!(
    benches,
    bench_new_feature,
    // ... other benchmarks
);
```

### 3. Run

```bash
cargo bench --manifest-path benches/Cargo.toml my_feature
```

## Troubleshooting

### "No benchmark targets found"

**Solution**: Use `--manifest-path`:
```bash
cargo bench --manifest-path benches/Cargo.toml
```

### Unstable Results

**Solutions**:
- Close background applications
- Keep laptop plugged in
- Disable CPU frequency scaling
- Increase sample size: `-- --sample-size 200`

### Test Files Not Found

**Solution**: Test files use absolute paths via `CARGO_MANIFEST_DIR`. If you see warnings, ensure files exist:
```bash
ls -lh benches/test_data/
```

### Out of Memory

**Solution**: Reduce image sizes or sample count:
```bash
cargo bench --manifest-path benches/Cargo.toml -- --sample-size 10
```

## Performance Tips

1. **Always measure first** - Profile before optimizing
2. **Use real data** - Synthetic data may not reflect actual workloads
3. **Trust statistics** - Only act on p < 0.05 results
4. **Consistent environment** - Same machine, power state, background load
5. **Multiple runs** - Run 3+ times to verify consistency

## Project Structure

```
benches/
├── Cargo.toml              # Benchmark dependencies (criterion 0.7.0)
├── README.md              # This file
├── CHANGELOG.md           # Update history
├── src/
│   └── lib.rs            # Test data generation utilities
├── benches/
│   └── kg_decode.rs      # KG decode benchmark suite
├── test_data/
│   ├── README.md         # Test file documentation
│   ├── BLACK             # Real game file (1.1 KB)
│   └── VYADOY01          # Real game file (126 KB)
└── run_benchmarks.sh     # Convenience script
```

## Resources

- [Criterion.rs Documentation](https://bheisler.github.io/criterion.rs/book/)
- [The Rust Performance Book](https://nnethercote.github.io/perf-book/)
- [Flamegraph](https://github.com/flamegraph-rs/flamegraph)
- [CHANGELOG.md](./CHANGELOG.md) - Recent updates and findings

## Contributing

When adding benchmarks:
1. Use real files when possible (from `bin/kg_extract/`)
2. Document expected performance characteristics
3. Include both best-case and worst-case scenarios
4. Update this README with new findings

---

**Version**: 1.0.0  
**Criterion**: 0.7.0  
**Status**: ✅ Production Ready  
**Maintainer**: dvine-rs team