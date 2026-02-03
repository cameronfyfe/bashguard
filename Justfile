_:
    @just --list

fmt:
    nixpkgs-fmt .
    cargo fmt
    cargo sort --grouped

fmt-check:
    nixpkgs-fmt --check .
    cargo fmt -- --check
    cargo sort --grouped --check

lint:
    cargo clippy -- -D warnings

build:
    cargo build

build-release:
    cargo build --release

test:
    cargo test

readme-update:
    present --in-place README.md

readme-check: _tmp
    present README.md > tmp/README.md
    diff README.md tmp/README.md

ci: readme-check fmt-check lint build test

_tmp:
    mkdir -p tmp
