#!/bin/bash

if [ $# -ne 3 ]; then
    echo "Usage: $0 <number_of_clients> <max_files> <user>"
    exit 1
fi

CLIENTS=$1
FILES=$2
USER=$3

echo "Deploying server and $CLIENTS clients..."

# Sync kafka-stream locally from git workspace to temp deploy directory.
echo "Syncing kafka-stream from git workspace..."
if [ ! -d "kafka-stream" ]; then
    echo "ERROR: kafka-stream directory not found in current directory"
    exit 1
fi

# Create a temporary deploy directory locally, then upload via deploy.py
DEPLOY_TEMP="kafka-stream-deploy-tmp"
rm -rf "$DEPLOY_TEMP"
if command -v rsync >/dev/null 2>&1; then
    rsync -a --delete kafka-stream/ "$DEPLOY_TEMP/"
else
    cp -r kafka-stream "$DEPLOY_TEMP"
fi
echo "kafka-stream synced to $DEPLOY_TEMP (ready to upload)"

echo "Deploying server..."
python3 scripts/deploy.py --user $USER --count 1 \
    --cmd "cd ~/4sl07/deploy/kafka-stream && sh scripts/config.sh && sh scripts/start-server.sh $FILES || sleep 100" \
    "$DEPLOY_TEMP"

HOST=$(cat deployed_hosts.txt)
echo $HOST

read -rp "Press ENTER to continue"

echo "Deploying clients..."
python3 scripts/deploy.py --user $USER --count $CLIENTS --append-hosts \
    --cmd "cd ~/4sl07/deploy/kafka-stream && sh scripts/start-client.sh $HOST:9092 || sleep 100" \
    "$DEPLOY_TEMP"

echo "Deployment complete."