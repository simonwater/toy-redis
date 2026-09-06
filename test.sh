#!/usr/bin/env bash

# 原有tester运行时需要传递json格式的关卡slug，想要验证的用例较多时比较麻烦。
# 此脚本的作用是支持类似官方线上的 --previous 模式，指定一个关卡，然后生成该关卡及以前所有关卡的json，再运行tester

# 指向你的 Go Tester 二进制文件路径（如果在当前目录下，直接填 ./tester）
TESTER_BIN="./tester_mac_x86.out"

# 按顺序定义 Shell 教程的所有 Stage Slug
STAGES=(
  "jm1" "rg2" "wy1" "zu2" "qq0" "la7"  # Base
)

TARGET_SLUG=$1
PREVIOUS=false

if [ "$2" == "--previous" ] || [ "$2" == "-p" ]; then
  PREVIOUS=true
fi

if [ -z "$TARGET_SLUG" ]; then
  # echo "Usage: ./test.sh <stage_slug> [--previous|-p]"
  # echo "Example: ./test.sh xk3 --previous"
  # exit 1
  TARGET_SLUG="wy1"
  PREVIOUS=true
fi

# 寻找目标 slug 在数组中的位置
TARGET_INDEX=-1
for i in "${!STAGES[@]}"; do
   if [[ "${STAGES[$i]}" == "${TARGET_SLUG}" ]]; then
       TARGET_INDEX=$i
       break
   fi
done

if [ $TARGET_INDEX -eq -1 ]; then
  echo "Error: Stage slug '${TARGET_SLUG}' not found."
  exit 1
fi

# 构建 JSON 关卡列表 (补全 title 字段)
JSON_STAGES="["
if [ "$PREVIOUS" = true ]; then
  # 包含从 0 到 TARGET_INDEX 的所有关卡
  for (( i=0; i<=$TARGET_INDEX; i++ )); do
    SLUG="${STAGES[$i]}"
    JSON_STAGES="${JSON_STAGES}{\"slug\":\"${SLUG}\",\"tester_log_prefix\":\"tester::#${SLUG}\",\"title\":\"Stage ${SLUG}\"}"
    if [ $i -lt $TARGET_INDEX ]; then
      JSON_STAGES="${JSON_STAGES},"
    fi
  done
else
  # 仅运行当前目标关卡
  JSON_STAGES="${JSON_STAGES}{\"slug\":\"${TARGET_SLUG}\",\"tester_log_prefix\":\"tester::#${TARGET_SLUG}\",\"title\":\"Stage ${TARGET_SLUG}\"}"
fi
JSON_STAGES="${JSON_STAGES}]"

# 禁用 rustup 自动更新并提前编译一次
export RUSTUP_AUTO_SELF_UPDATE=0
cargo build --quiet

# 执行测试
CODECRAFTERS_REPOSITORY_DIR="$(pwd)" \
CODECRAFTERS_TEST_CASES_JSON="${JSON_STAGES}" \
$TESTER_BIN