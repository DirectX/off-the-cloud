#!/bin/bash

RUST_LOG=info ./target/release/off-the-cloud imap pull --email=$1 --password=$2
