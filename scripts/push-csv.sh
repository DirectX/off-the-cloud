#!/bin/bash

FILENAME="$1"

if [[ -z "$FILENAME" ]]; then
  >&2 echo "Usage: $0 <filename>"
  exit 1
fi

python3 - <<EOF
import csv
import sys
import subprocess

filename = "$FILENAME"
try:
    with open(filename, newline='') as csvfile:
        reader = csv.reader(csvfile)
        for row in reader:
            email = row[0]
            password = row[1]
            print(f"Email: {email}")
            subprocess.run(["./scripts/push.sh", email, password]) 

except FileNotFoundError:
    print(f"File {filename} not found.", file=sys.stderr)
EOF
