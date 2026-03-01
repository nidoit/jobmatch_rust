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
  run             Package CLI as self-extracting .run installer
  run-gui         Package GUI as self-extracting .run installer
  run-all         Package both CLI and GUI into a single .run installer
  test            Run all tests
  clean           Clean build artifacts
  check           Check dependencies
  help            Show this help

Examples:
  ./build.sh              # Build CLI (debug)
  ./build.sh gui          # Build GUI (debug)
  ./build.sh release      # Build both (release, optimized)
  ./build.sh run          # Package CLI as jobmatch-0.2.0.run
  ./build.sh run-all      # Package both as jobmatch-0.2.0.run
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
    rm -rf dist/
    echo -e "${GREEN}Clean complete.${NC}"
}

# Read version from Cargo.toml
get_version() {
    grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/'
}

build_run() {
    local bins="${1:-cli}"  # cli, gui, or all
    local version
    version="$(get_version)"
    local run_file="dist/jobmatch-${version}.run"
    local staging_dir
    staging_dir="$(mktemp -d)"

    check_deps

    echo -e "${CYAN}Building release binaries for .run package...${NC}"

    # Build requested binaries
    if [ "$bins" = "cli" ] || [ "$bins" = "all" ]; then
        build_cli release
        cp target/release/jobmatch "$staging_dir/"
    fi
    if [ "$bins" = "gui" ] || [ "$bins" = "all" ]; then
        build_gui release
        cp target/release/jobmatch-gui "$staging_dir/"
        # Include UI assets for the GUI
        if [ -d ui ]; then
            cp -r ui "$staging_dir/ui"
        fi
    fi

    # Include icons
    if [ -d icons ]; then
        cp -r icons "$staging_dir/icons"
    fi

    echo -e "${CYAN}Packaging .run installer...${NC}"

    mkdir -p dist

    # Create the compressed archive
    local archive
    archive="$(mktemp)"
    tar czf "$archive" -C "$staging_dir" .

    # Write self-extracting header
    cat > "$run_file" <<'HEADER'
#!/usr/bin/env bash
set -euo pipefail

# JobMatch self-extracting installer
INSTALL_DIR="${JOBMATCH_INSTALL_DIR:-$HOME/.local/share/jobmatch}"
BIN_DIR="${JOBMATCH_BIN_DIR:-$HOME/.local/bin}"

echo "JobMatch Installer"
echo "=================="
echo ""
echo "Install directory: $INSTALL_DIR"
echo "Binary links:      $BIN_DIR"
echo ""

# Find the archive offset (line after PAYLOAD marker)
ARCHIVE_LINE=$(awk '/^__PAYLOAD__$/{print NR + 1; exit 0;}' "$0")
if [ -z "$ARCHIVE_LINE" ]; then
    echo "Error: corrupt installer (missing payload marker)." >&2
    exit 1
fi

mkdir -p "$INSTALL_DIR" "$BIN_DIR"

# Extract payload
tail -n +"$ARCHIVE_LINE" "$0" | tar xzf - -C "$INSTALL_DIR"

# Symlink binaries into BIN_DIR
for bin in jobmatch jobmatch-gui; do
    if [ -f "$INSTALL_DIR/$bin" ]; then
        chmod +x "$INSTALL_DIR/$bin"
        ln -sf "$INSTALL_DIR/$bin" "$BIN_DIR/$bin"
        echo "  Installed: $BIN_DIR/$bin"
    fi
done

echo ""
echo "Installation complete!"
echo "Make sure $BIN_DIR is in your PATH."
exit 0
__PAYLOAD__
HEADER

    # Append the archive after the payload marker
    cat "$archive" >> "$run_file"
    chmod +x "$run_file"

    # Cleanup
    rm -rf "$staging_dir" "$archive"

    local size
    size="$(du -h "$run_file" | cut -f1)"
    echo ""
    echo -e "${GREEN}.run package created: ${CYAN}${run_file}${NC} (${size})"
    echo -e "  Run with: ${CYAN}./${run_file}${NC}"
    echo -e "  Custom install dir: ${CYAN}JOBMATCH_INSTALL_DIR=/opt/jobmatch ./${run_file}${NC}"
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
    run)
        build_run cli
        ;;
    run-gui)
        build_run gui
        ;;
    run-all)
        build_run all
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
