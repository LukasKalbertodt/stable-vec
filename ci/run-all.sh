#!/bin/bash

# Exit script on the first error
set -o errexit -o nounset

MY_PATH="`dirname \"$0\"`"

# basic style check
$MY_PATH/check-basic-style.sh

# check that everything compiles and all tests pass
$MY_PATH/compile-test-all.sh

echo "++++++++++++++++++++++++++++++++++++++++++++++++++++"
echo "+              Everything is fine!                 +"
echo "++++++++++++++++++++++++++++++++++++++++++++++++++++"
