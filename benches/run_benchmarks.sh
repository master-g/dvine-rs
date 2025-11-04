#!/usr/bin/env bash
# Benchmark runner script for dvine-rs
#
# Usage:
#   ./benches/run_benchmarks.sh [OPTIONS]
#
# Options:
#   --all           Run all benchmarks (default)
#   --kg            Run only KG decode benchmarks
#   --profile       Run with profiling enabled
#   --flamegraph    Generate flamegraph
#   --baseline NAME Save results as baseline
#   --compare NAME  Compare against baseline
#   --report        Open HTML report after completion
#   --help          Show this help message

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Default options
RUN_ALL=true
RUN_KG=false
PROFILE=false
FLAMEGRAPH=false
BASELINE=""
COMPARE=""
OPEN_REPORT=false

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --all)
            RUN_ALL=true
            shift
            ;;
        --kg)
            RUN_ALL=false
            RUN_KG=true
            shift
            ;;
        --profile)
            PROFILE=true
            shift
            ;;
        --flamegraph)
            FLAMEGRAPH=true
            shift
            ;;
        --baseline)
            BASELINE="$2"
            shift 2
            ;;
        --compare)
            COMPARE="$2"
            shift 2
            ;;
        --report)
            OPEN_REPORT=true
            shift
            ;;
        --help)
            echo "Benchmark runner for dvine-rs"
            echo ""
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --all           Run all benchmarks (default)"
            echo "  --kg            Run only KG decode benchmarks"
            echo "  --profile       Run with profiling enabled"
            echo "  --flamegraph    Generate flamegraph (requires cargo-flamegraph)"
            echo "  --baseline NAME Save results as baseline"
            echo "  --compare NAME  Compare against baseline"
            echo "  --report        Open HTML report after completion"
            echo "  --help          Show this help message"
            echo ""
            echo "Examples:"
            echo "  $0 --all --report"
            echo "  $0 --kg --baseline before"
            echo "  $0 --kg --compare before"
            echo "  $0 --flamegraph --kg"
            exit 0
            ;;
        *)
            echo -e "${RED}Unknown option: $1${NC}"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

# Print header
echo -e "${BLUE}╔════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║  dvine-rs Benchmark Suite             ║${NC}"
echo -e "${BLUE}╔════════════════════════════════════════╗${NC}"
echo ""

# Check for required tools
if [ "$FLAMEGRAPH" = true ]; then
    if ! command -v cargo-flamegraph &> /dev/null; then
        echo -e "${YELLOW}Warning: cargo-flamegraph not found${NC}"
        echo "Install with: cargo install flamegraph"
        exit 1
    fi

    # Check if we're on macOS and need sudo
    if [[ "$OSTYPE" == "darwin"* ]]; then
        echo -e "${YELLOW}Note: Flamegraph on macOS may require sudo${NC}"
        # Check if Xcode is available (not required for flamegraph)
        if ! xcode-select -p &> /dev/null; then
            echo -e "${YELLOW}Xcode Command Line Tools detected (flamegraph will work)${NC}"
        fi
    fi
fi

# Build benchmark targets first
echo -e "${GREEN}Building benchmark targets...${NC}"
cargo build --release --manifest-path benches/Cargo.toml

# Construct benchmark command
BENCH_CMD="cargo bench --manifest-path benches/Cargo.toml"

# Add baseline options
if [ -n "$BASELINE" ]; then
    BENCH_CMD="$BENCH_CMD --save-baseline $BASELINE"
    echo -e "${YELLOW}Saving baseline: $BASELINE${NC}"
fi

if [ -n "$COMPARE" ]; then
    BENCH_CMD="$BENCH_CMD --baseline $COMPARE"
    echo -e "${YELLOW}Comparing against baseline: $COMPARE${NC}"
fi

# Add profiling options
if [ "$PROFILE" = true ]; then
    BENCH_CMD="$BENCH_CMD -- --profile-time=5"
    echo -e "${YELLOW}Profiling enabled (5 seconds per benchmark)${NC}"
fi

# Run flamegraph if requested
if [ "$FLAMEGRAPH" = true ]; then
    echo -e "${GREEN}Generating flamegraph...${NC}"

    if [ "$RUN_KG" = true ] || [ "$RUN_ALL" = true ]; then
        # Try with sudo on macOS, without on Linux
        if [[ "$OSTYPE" == "darwin"* ]]; then
            echo -e "${YELLOW}Running with sudo (required on macOS)...${NC}"
            sudo -E cargo flamegraph --bench kg_decode --manifest-path benches/Cargo.toml -o flamegraph_kg.svg
        else
            cargo flamegraph --bench kg_decode --manifest-path benches/Cargo.toml -o flamegraph_kg.svg
        fi

        if [ $? -eq 0 ]; then
            echo -e "${GREEN}✓ Flamegraph saved to: flamegraph_kg.svg${NC}"
        else
            echo -e "${RED}✗ Flamegraph generation failed${NC}"
            echo -e "${YELLOW}Try: sudo cargo flamegraph --bench kg_decode --manifest-path benches/Cargo.toml${NC}"
        fi
    fi

    exit 0
fi

# Run benchmarks
echo ""
echo -e "${GREEN}Running benchmarks...${NC}"
echo -e "${BLUE}═══════════════════════════════════════${NC}"
echo ""

if [ "$RUN_ALL" = true ]; then
    echo -e "${YELLOW}Running all benchmarks...${NC}"
    eval $BENCH_CMD
elif [ "$RUN_KG" = true ]; then
    echo -e "${YELLOW}Running KG decode benchmarks...${NC}"
    eval $BENCH_CMD kg_
fi

# Print summary
echo ""
echo -e "${BLUE}═══════════════════════════════════════${NC}"
echo -e "${GREEN}✓ Benchmarks completed!${NC}"
echo ""

# Show report location
REPORT_DIR="target/criterion"
if [ -d "$REPORT_DIR" ]; then
    echo -e "${YELLOW}Reports available at:${NC}"
    echo "  HTML: $REPORT_DIR/report/index.html"
    echo ""

    # Open report if requested
    if [ "$OPEN_REPORT" = true ]; then
        echo -e "${GREEN}Opening report in browser...${NC}"
        if command -v open &> /dev/null; then
            open "$REPORT_DIR/report/index.html"
        elif command -v xdg-open &> /dev/null; then
            xdg-open "$REPORT_DIR/report/index.html"
        else
            echo -e "${YELLOW}Could not open browser automatically${NC}"
            echo "Please open: $REPORT_DIR/report/index.html"
        fi
    fi
fi

# Show hot path tips
echo -e "${BLUE}═══════════════════════════════════════${NC}"
echo -e "${YELLOW}Hot Path Analysis Tips:${NC}"
echo ""
echo "1. Check HTML reports for detailed timing breakdown"
echo "2. Use flamegraph for visual profiling:"
echo -e "   ${GREEN}$0 --flamegraph --kg${NC}"
echo ""
echo "3. Compare before/after optimizations:"
echo -e "   ${GREEN}$0 --baseline before${NC}"
echo -e "   ${GREEN}# Make changes...${NC}"
echo -e "   ${GREEN}$0 --compare before${NC}"
echo ""
echo "4. For detailed profiling (Linux):"
echo -e "   ${GREEN}perf record --call-graph dwarf target/release/deps/kg_decode-*${NC}"
echo -e "   ${GREEN}perf report${NC}"
echo ""
echo "5. For detailed profiling (macOS with Xcode):"
echo -e "   ${GREEN}instruments -t 'Time Profiler' target/release/deps/kg_decode-*${NC}"
echo -e "   ${YELLOW}Note: Requires full Xcode installation${NC}"
echo ""
echo "6. Cross-platform profiling (recommended for macOS without Xcode):"
echo -e "   ${GREEN}Use flamegraph (option --flamegraph)${NC}"
echo ""

echo -e "${BLUE}╚════════════════════════════════════════╝${NC}"
