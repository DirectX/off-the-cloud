#!/bin/bash

if [ $# -ne 2 ]; then
    >&2 echo "Usage: ./scripts/pull.sh email password"
    exit 1
fi

off-the-cloud imap pull --email=$1 --password=$2
