#!/usr/bin/env bash
# tests/images/create-images.sh
# Creates test NTFS disk images using mkntfs (from ntfs-3g) if available.
# These images are checked into the repo (via Git LFS) for reproducible testing.

set -euo pipefail

IMAGES_DIR="$(cd "$(dirname "$0")" && pwd)"

check_mkntfs() {
    if ! command -v mkntfs &>/dev/null; then
        echo "mkntfs not found. Install ntfs-3g:"
        echo "  brew install ntfs-3g-mac"
        echo "  # or: brew install ntfs-3g"
        echo ""
        echo "Alternatively, create images manually on Windows."
        echo "See README.md for instructions."
        exit 1
    fi
}

create_basic_image() {
    echo "=== Creating basic NTFS image (16MB) ==="
    local img="$IMAGES_DIR/basic.img"
    dd if=/dev/zero of="$img" bs=1M count=16 2>/dev/null
    mkntfs -F -L "Basic" "$img"
    echo "Created $img"
    echo "NOTE: You must add test files manually (hello.txt, Documents/readme.txt)"
    echo "      Mount on Windows or use ntfs-3g tools to add files."
}

create_empty_image() {
    echo "=== Creating empty NTFS image (8MB) ==="
    local img="$IMAGES_DIR/empty.img"
    dd if=/dev/zero of="$img" bs=1M count=8 2>/dev/null
    mkntfs -F -L "Empty" "$img"
    echo "Created $img"
}

main() {
    check_mkntfs

    case "${1:-all}" in
        basic) create_basic_image ;;
        empty) create_empty_image ;;
        all)
            create_basic_image
            create_empty_image
            echo ""
            echo "=== dirty.img and deep-paths.img require manual creation ==="
            echo "See README.md for instructions."
            ;;
        *)
            echo "Usage: $0 [basic|empty|all]"
            exit 1
            ;;
    esac
}

main "$@"
