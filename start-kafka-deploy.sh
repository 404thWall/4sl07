#!/bin/bash

if [ $# -ne 3 ]; then
    echo "Usage: $0 <number_of_clients> <max_files> <user>"
    exit 1
fi

CLIENTS=$1
FILES=$2
USER=$3
echo "Deploying server and $CLIENTS clients..."

echo "Deploying server..."
python3 scripts/deploy.py --user $USER --count 1 --cmd "cd ~/4sl07/deploy/kafka-stream && sh scripts/config.sh && sh scripts/start-server.sh $FILES || sleep 100" --no-scp kafka-stream.zip

HOST=$(cat deployed_hosts.txt)
echo $HOST

read -rp "Press ENTER to continue"

echo "Deploying clients..."
python3 scripts/deploy.py --user $USER --count $CLIENTS --append-hosts --cmd "cd ~/4sl07/deploy/kafka-stream && sh scripts/start-client.sh $HOST:9092 || sleep 100" --no-scp kafka-stream.zip

echo "Deployment complete."