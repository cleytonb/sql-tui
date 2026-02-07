#!/bin/bash
cd "$(dirname "$0")"
source .env 2>/dev/null
./target/release/sqltui
