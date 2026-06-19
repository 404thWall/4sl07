#!/bin/bash

if [ $# -ne 1 ]; then
    echo "Usage: $0 <user>"
fi

USER=$1
while read host; do
    echo "Cleaning $host..."
    ssh -n -o ConnectTimeout=5 $USER@$host "pkill -f WordCountApplication; rm -rf /tmp/kafka-streams" &
done < deployed_hosts.txt
wait