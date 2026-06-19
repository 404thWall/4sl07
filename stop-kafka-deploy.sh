#!/bin/bash

if [ $# -ne 1 ]; then
    echo "Usage: $0 <user>"
fi

USER=$1
python3 scripts/deploy.py --kill --user $USER
