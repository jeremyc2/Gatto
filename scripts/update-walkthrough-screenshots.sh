#!/bin/sh

set -eu

PROJECT_DIRECTORY=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
OUTPUT_DIRECTORY="$PROJECT_DIRECTORY/docs/images/walkthrough"

cd "$PROJECT_DIRECTORY"
SEED=0 cargo run --quiet --features walkthrough-screenshots --bin walkthrough-screenshots -- "$OUTPUT_DIRECTORY"
