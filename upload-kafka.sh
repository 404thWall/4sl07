#!/bin/bash
set -e

if [ $# -lt 1 ]; then
    echo "Usage: $0 <user>"
    exit 1
fi

USER=$1
HOST="tp-1a260-11"
REMOTE_DIR="~/4sl07/deploy"
ARCHIVE="kafka-stream.zip"

zip -r "$ARCHIVE" kafka-stream
ssh "$USER@$HOST" "mkdir -p $REMOTE_DIR"
scp "$ARCHIVE" "$USER@$HOST:$REMOTE_DIR/"
ssh "$USER@$HOST" "cd $REMOTE_DIR && unzip -o $ARCHIVE"

