#!/bin/bash

if [ $# -ne 2 ]; then
    >&2 echo "Usage: $0 <email> <password>"
    exit 1
fi

off-the-cloud imap push --email=$1 --password=$2
