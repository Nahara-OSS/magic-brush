#!/bin/bash

# Cleaning up
if [ -d out ]; then
    echo "Cleaning up out/ directory"
    rm -rf out/*
fi

# Build
wasm-pack build
deno bundle \
    --unstable-raw-imports \
    --minify \
    --outdir out \
    index.html