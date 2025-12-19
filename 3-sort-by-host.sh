#!/bin/bash

# 配置
INPUT_DIR="checked"
OUTPUT_DIR="sorted"

echo "开始按 URL host 排序处理..."

# 清理并创建输出目录
rm -fr "$OUTPUT_DIR"
mkdir -p "$OUTPUT_DIR"

# 检查输入目录是否存在
if [ ! -d "$INPUT_DIR" ]; then
    echo "错误：输入目录 '$INPUT_DIR' 不存在！"
    exit 1
fi

# 处理输入目录中的所有txt文件
for file in "$INPUT_DIR"/*.txt; do
    if [ -f "$file" ]; then
        filename=$(basename "$file")
        output_file="$OUTPUT_DIR/$filename"

        echo "处理文件: $filename"

        # 创建临时文件用于排序
        temp_file="${file}.tmp"

        # 处理每一行，提取host
        temp_extract="${file}.extract"
        while IFS= read -r line; do
            # 跳过空行
            if [ -z "$line" ]; then
                continue
            fi

            # 提取URL部分（逗号后的部分）
            if [[ "$line" =~ ,(.+)$ ]]; then
                url="${BASH_REMATCH[1]}"

                # 提取host
                if [[ "$url" =~ https?://([^/]+) ]]; then
                    host="${BASH_REMATCH[1]}"
                    # 输出格式：host|原始行
                    echo "$host|$line"
                else
                    # 如果无法提取host，使用默认值
                    echo "unknown|$line"
                fi
            else
                # 如果格式不正确，使用默认值
                echo "unknown|$line"
            fi
        done < "$file" > "$temp_extract"

        # 统计每个host的数量并按数量倒序排列
        # 首先统计每个host的出现次数
        awk -F'|' '{count[$1]++} END {for (host in count) print count[host], host}' "$temp_extract" | sort -nr > "${file}.host_count"

        # 根据host数量倒序排列原始数据
        while IFS= read -r count_host; do
            host=$(echo "$count_host" | cut -d' ' -f2-)
            # 提取该host的所有行
            grep "^$host|" "$temp_extract" | cut -d'|' -f2-
        done < "${file}.host_count" > "$temp_file"

        # 清理临时文件
        rm -f "$temp_extract" "${file}.host_count"

        # 移动临时文件到输出目录
        if [ -f "$temp_file" ]; then
            mv "$temp_file" "$output_file"

            # 统计行数
            total_lines=$(wc -l < "$output_file" 2>/dev/null || echo "0")
            echo "✓ 排序完成: $filename ($total_lines 行)"
        else
            echo "⚠ 处理失败: $filename"
        fi
    fi
done

echo ""
echo "所有文件排序完成！"
echo ""
echo "处理结果："
echo "  原始文件目录: $INPUT_DIR/"
echo "  排序后文件目录: $OUTPUT_DIR/"