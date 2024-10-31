#!/bin/bash

DOMAIN="$1"

if [[ -z "$DOMAIN" ]]; then
  >&2 echo "Usage: $0 <domain>"
  exit 1
fi

tar -cvzf ./messages/$(date +%Y%m%d_%H%M%S)-${DOMAIN}-backup.tar.gz ./messages/${DOMAIN}