#!/bin/bash

CLIENTS=3

BASE_PATH=$(cat COMMON_PATH | tr -d '\r')
echo "Base path: $BASE_PATH"

if [ $# -ne 2 ]; then
    echo "Usage: $0 <number_of_clients> <user>"
    exit 1
fi

CLIENTS=$1
USER=$2

if [ "$CLIENTS" = "kill" ]; then
    echo "Killing all clients and server..."
    python3 scripts/deploy.py --user $USER --count 1 --kill ./target/release/slr07
    exit 0
fi

cargo build --release --features prod

echo "Deploying server and $CLIENTS clients..."

echo "Deploying server..."
python3 scripts/deploy.py --user $USER --count 1 --cmd "mkdir -p $BASE_PATH && ./4sl07/deploy/slr07 server 2>&1 | tee $BASE_PATH/server.log" ./target/release/slr07

HOST=$(cat deployed_hosts.txt)
echo $HOST

echo "Deploying clients..."
python3 scripts/deploy.py --user $USER --count $CLIENTS --append-hosts --cmd "mkdir -p $BASE_PATH && ./4sl07/deploy/slr07 client 9001 $HOST.enst.fr $USER 2>&1 | tee $BASE_PATH/client.log" --no-scp ./target/release/slr07

echo "Deployment complete."
