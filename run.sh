#!/bin/bash
cd "$(dirname "$0")"
set -a
source .env 2>/dev/null
set +a
./target/release/alrajhi_sql_tui "$@"
