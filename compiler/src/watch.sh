#!/bin/bash

INPUT_FILE="$1"
OUTPUT_FILE=$(mktemp)

if [ -z "$INPUT_FILE" ]; then
  echo "Usage: watch [INPUT]"
  exit 1
fi

clear
echo "Watching $INPUT_FILE for changes and displaying output..."

cleanup() {
  rm -f "$OUTPUT_FILE"
  exit
}

trap cleanup SIGINT SIGTERM

fswatch -r 0.1 -o "$INPUT_FILE" | while read -r; do
  ../../target/debug/compiler "$INPUT_FILE" "$OUTPUT_FILE"
  clear
  cat "$OUTPUT_FILE"
done
