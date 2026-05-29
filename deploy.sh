#!/bin/bash

cargo build --release --features prod
CLIENTS=3

if [ $# -ne 0 ]; then
    CLIENTS=$1
fi

echo "Deploying server and $CLIENTS clients..."

echo "Deploying server..."
python3 scripts/deploy.py --user theinrich-24 --count 1 --cmd "mkdir -p /tmp/4sl07_grp3 && ./4sl07/deploy/slr07 server 2>&1 | tee /tmp/4sl07_grp3/server.log" ./target/release/slr07

HOST=$(cat deployed_hosts.txt)
echo $HOST

echo "Deploying clients..."
python3 scripts/deploy.py --user theinrich-24 --count $CLIENTS --append-hosts --cmd "mkdir -p /tmp/4sl07_grp3 && ./4sl07/deploy/slr07 client 9001 $HOST.enst.fr 2>&1 | tee /tmp/4sl07_grp3/client.log" --no-scp ./target/release/slr07 &

echo "Deployment complete."
