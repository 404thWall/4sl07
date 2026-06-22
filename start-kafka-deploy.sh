#!/bin/bash

if [ $# -ne 3 ]; then
    echo "Usage: $0 <number_of_clients> <max_files> <user>"
    exit 1
fi

CLIENTS=$1
FILES=$2
USER=$3

# Sync latest kafka-stream from remote git workspace to deploy directory.
REMOTE_SYNC_CMD='SRC=""; for d in "$HOME/4sl07_src/4sl07/kafka-stream" "$HOME/4sl07-src/4sl07/kafka-stream" "$HOME/4sl07_src/kafka-stream" "$HOME/4sl07-src/kafka-stream"; do [ -d "$d" ] && SRC="$d" && break; done; [ -n "$SRC" ] || { echo "ERROR: kafka-stream source not found under ~/4sl07_src or ~/4sl07-src"; exit 1; }; mkdir -p "$HOME/4sl07/deploy"; if command -v rsync >/dev/null 2>&1; then rsync -a --delete "$SRC/" "$HOME/4sl07/deploy/kafka-stream/"; else rm -rf "$HOME/4sl07/deploy/kafka-stream"; cp -r "$SRC" "$HOME/4sl07/deploy/kafka-stream"; fi'

echo "Deploying server and $CLIENTS clients..."

echo "Deploying server..."
python3 scripts/deploy.py --user $USER --count 1 --cmd "$REMOTE_SYNC_CMD && cd ~/4sl07/deploy/kafka-stream && sh scripts/config.sh && sh scripts/start-server.sh $FILES || sleep 100" --no-scp kafka-stream.zip

HOST=$(cat deployed_hosts.txt)
echo $HOST

read -rp "Press ENTER to continue"

echo "Deploying clients..."
python3 scripts/deploy.py --user $USER --count $CLIENTS --append-hosts --cmd "$REMOTE_SYNC_CMD && cd ~/4sl07/deploy/kafka-stream && sh scripts/start-client.sh $HOST:9092 || sleep 100" --no-scp kafka-stream.zip

echo "Deployment complete."