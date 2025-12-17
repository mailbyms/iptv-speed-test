#!/bin/bash

# 批量测试脚本
# 使用示例：./batch_test.sh test_urls.txt

if [ $# -eq 0 ]; then
    echo "使用方法: $0 <urls_file>"
    echo "示例: $0 test_urls.txt"
    exit 1
fi

URLS_FILE=$1

if [ ! -f "$URLS_FILE" ]; then
    echo "错误: 文件 $URLS_FILE 不存在"
    exit 1
fi

echo "开始批量测试..."
echo "URL文件: $URLS_FILE"
echo "========================================="

if [ $? -ne 0 ]; then
    echo "错误: 项目构建失败"
    exit 1
fi

# 读取URL文件并测试
while IFS= read -r url || [[ -n "$url" ]]; do
    # 跳过注释和空行（包括只有空白字符的行）
    # 使用更强的空行检测：移除所有空白字符后检查是否为空
    if [[ $url =~ ^[[:space:]]*# ]] || [[ -z "${url// }" ]] || [[ -z "$(echo "$url" | tr -d ' \t\n\r')" ]]; then
        continue
    fi

    echo ""
    echo "测试 URL: $url"
    echo "----------------------------------------"

    # 执行测试
    timeout 10s ./target/release/iptv-speed-test "$url" --verbose
    exit_code=$?

    if [ $exit_code -eq 124 ]; then
        echo "测试超时（60秒）"
    elif [ $exit_code -ne 0 ]; then
        echo "测试失败，退出码: $exit_code"
    fi

done < "$URLS_FILE"

echo "========================================="
echo "批量测试完成！"