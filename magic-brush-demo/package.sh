#!/bin/bash

# Cleaning up
if [ -d out ]; then
    echo "Cleaning up out/ directory"
    rm -rf out/*
fi

# Build
wasm-pack \
    build \
    --release

deno bundle \
    --unstable-raw-imports \
    --minify \
    --sourcemap \
    --outdir out \
    index.html
