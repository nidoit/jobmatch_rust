#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

usage() {
    cat <<EOF

${CYAN}JobMatch Build Script${NC}

Usage: ./build.sh [command]

Commands:
  cli             Build CLI binary only (default)
  gui             Build Tauri GUI binary
  all             Build both CLI and GUI
  release         Build release binaries (optimized)
  test            Run all tests
  clean           Clean build artifacts
  check           Check dependencies
  help            Show this help

Examples:
  ./build.sh              # Build CLI (debug)
  ./build.sh gui          # Build GUI (debug)
  ./build.sh release      # Build both (release, optimized)
  ./build.sh test         # Run tests

EOF
}

check_deps() {
    echo -e "${CYAN}Checking dependencies...${NC}"

    if ! command -v cargo &>/dev/null; then
        echo -e "${RED}Error: cargo not found. Install Rust: https://rustup.rs${NC}"
        exit 1
    fi
    echo -e "  ${GREEN}cargo$(NC) $(cargo --version)"

    if ! command -v rustc &>/dev/null; then
        echo -e "${RED}Error: rustc not found.${NC}"
        exit 1
    fi
    echo -e "  ${GREEN}rustc${NC} $(rustc --version)"

    # Check GUI deps (Linux only)
    if [[ "$OSTYPE" == "linux-gnu"* ]]; then
        local missing=()
        for lib in webkit2gtk-4.1 gtk+-3.0; do
            if ! pkg-config --exists "$lib" 2>/dev/null; then
                missing+=("$lib")
            fi
        done

        if [ ${#missing[@]} -gt 0 ]; then
            echo -e "${YELLOW}Warning: Missing GUI libraries: ${missing[*]}${NC}"
            echo -e "${YELLOW}Install with: sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev libsoup-3.0-dev${NC}"
            echo -e "${YELLOW}GUI build will fail without these. CLI build will work fine.${NC}"
        else
            echo -e "  ${GREEN}GUI deps${NC} OK (webkit2gtk, gtk)"
        fi
    fi

    echo -e "${GREEN}All core dependencies OK${NC}"
    echo ""
}

build_cli() {
    local mode="${1:-debug}"
    echo -e "${CYAN}Building CLI ($mode)...${NC}"

    if [ "$mode" = "release" ]; then
        cargo build --release --bin jobmatch
        echo -e "${GREEN}CLI binary: target/release/jobmatch${NC}"
    else
        cargo build --bin jobmatch
        echo -e "${GREEN}CLI binary: target/debug/jobmatch${NC}"
    fi
}

build_gui() {
    local mode="${1:-debug}"
    echo -e "${CYAN}Building GUI ($mode)...${NC}"

    if [ "$mode" = "release" ]; then
        cargo build --release --bin jobmatch-gui
        echo -e "${GREEN}GUI binary: target/release/jobmatch-gui${NC}"
    else
        cargo build --bin jobmatch-gui
        echo -e "${GREEN}GUI binary: target/debug/jobmatch-gui${NC}"
    fi
}

run_tests() {
    echo -e "${CYAN}Running tests...${NC}"
    cargo test -- --nocapture
    echo -e "${GREEN}All tests passed!${NC}"
}

do_clean() {
    echo -e "${CYAN}Cleaning build artifacts...${NC}"
    cargo clean
    echo -e "${GREEN}Clean complete.${NC}"
}

# ─── Main ───────────────────────────────────────────────────────────────

CMD="${1:-cli}"

case "$CMD" in
    cli)
        check_deps
        build_cli debug
        ;;
    gui)
        check_deps
        build_gui debug
        ;;
    all)
        check_deps
        build_cli debug
        build_gui debug
        ;;
    release)
        check_deps
        echo -e "${CYAN}Building release binaries (this may take a while)...${NC}"
        build_cli release
        build_gui release
        echo ""
        echo -e "${GREEN}Release build complete!${NC}"
        echo -e "  CLI: ${CYAN}target/release/jobmatch${NC}"
        echo -e "  GUI: ${CYAN}target/release/jobmatch-gui${NC}"
        ;;
    test)
        run_tests
        ;;
    clean)
        do_clean
        ;;
    check)
        check_deps
        ;;
    help|--help|-h)
        usage
        ;;
    *)
        echo -e "${RED}Unknown command: $CMD${NC}"
        usage
        exit 1
        ;;
esac
