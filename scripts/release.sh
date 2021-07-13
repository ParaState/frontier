#!/bin/bash

set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"

cd $SCRIPT_DIR
rm -rf release parastate-node.tar.gz
mkdir release
cp ../target/release/frontier-template-node release/parastate-node
cp $(find ../target -name libssvm-evmc.so) release
strip release/parastate-node
strip release/libssvm-evmc.so

cd release
tar zcf parastate-node.tar.gz parastate-node libssvm-evmc.so
echo SHA1:
sha1sum parastate-node libssvm-evmc.so | sed -E 's/[0-9a-f]{40}/`\0`/'
mv parastate-node.tar.gz $SCRIPT_DIR
echo
echo "Generated release tarball at $SCRIPT_DIR/parastate-node.tar.gz"
rm -rf $SCRIPT_DIR/release
