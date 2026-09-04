#!/bin/sh
#

# 直接启动前面在 test.sh 中编译好的二进制文件，只适用于本地编译tester的情况
exec $(dirname $0)/target/debug/toy-redis "$@"