# bashguard

`bashguard` is a simple guardrail system for bash calls from your coding agents.

# Quick Start

## Install

```bash
cargo install bashguard
```

## Setup

In your coding agent workspace:

```bash
bashguard init
```

# Justfile

```present just --list
Available recipes:
    build
    build-release
    ci
    fmt
    fmt-check
    lint
    readme-check
    readme-update
    test
```
