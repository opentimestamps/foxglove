#!/usr/bin/env bash

echo "Hello World!">a
ots -v stamp -m 1 -c http://localhost:1337/digest a & ./post.sh & ./post.sh & ./post.sh & ./post.sh
ots -v info a.ots
ots -v verify a.ots
rm a
rm a.ots
