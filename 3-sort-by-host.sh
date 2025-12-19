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

# 步骤1：合并所有文件为一个文件
echo "正在合并 INPUT_DIR 中的所有文件..."
# 合并后的文件名
merged_file="$OUTPUT_DIR/merged_file.txt"
# 生成输出文件名（基于合并文件名）
output_file="$OUTPUT_DIR/merged_file_sorted.txt"

# 将文件内容追加到合并文件
cat "$INPUT_DIR"/* | sort | uniq > "$merged_file"
echo "  合并文件: $merged_file"

# 步骤2：对合并后的文件按host排序分组
echo ""
echo "开始按 host 排序分组..."

# 创建临时文件用于排序
temp_file="${merged_file}.tmp"

# 处理每一行，提取host
temp_extract="${merged_file}.extract"
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
done < "$merged_file" > "$temp_extract"

# 统计每个host的数量并按数量倒序排列
# 首先统计每个host的出现次数
awk -F'|' '{count[$1]++} END {for (host in count) print count[host], host}' "$temp_extract" | sort -nr > "${merged_file}.host_count"

# 根据host数量倒序排列原始数据
while IFS= read -r count_host; do
    host=$(echo "$count_host" | cut -d' ' -f2-)
    # 提取该host的所有行
    grep "^$host|" "$temp_extract" | cut -d'|' -f2-
done < "${merged_file}.host_count" > "$temp_file"

# 清理临时文件
rm -f "$temp_extract" "${merged_file}.host_count"

# 移动临时文件到输出目录
if [ -f "$temp_file" ]; then
    mv "$temp_file" "$output_file"

    # 统计行数
    total_lines=$(wc -l < "$output_file" 2>/dev/null || echo "0")
    echo "✓ 排序完成: $output_file ($total_lines 行)"

    # 删除合并文件
    rm -f "$merged_file"
    echo "  已清理临时合并文件"

    # 步骤3：转换为M3U格式
    echo ""
    echo "开始转换为 M3U 格式..."

    m3u_output_file="${output_file%.txt}.m3u"

    # 检查排序文件是否存在
    if [ -f "$output_file" ]; then
        echo "正在转换: $output_file -> $m3u_output_file"

        # 创建M3U文件头部
        echo "#EXTM3U" > "$m3u_output_file"

        # 转换每一行
        converted_lines=0
        while IFS= read -r line; do
            # 跳过空行
            if [ -z "$line" ]; then
                continue
            fi

            # 提取频道名称和URL（格式：频道名,URL）
            if [[ "$line" =~ ^([^,]+),(.+)$ ]]; then
                channel_name="${BASH_REMATCH[1]}"
                url="${BASH_REMATCH[2]}"

                # 添加M3U格式的频道信息
                echo "#EXTINF:-1,$channel_name" >> "$m3u_output_file"
                echo "$url" >> "$m3u_output_file"
                converted_lines=$((converted_lines + 1))
            else
                # 如果格式不正确，尝试作为纯URL处理
                echo "#EXTINF:-1,$line" >> "$m3u_output_file"
                echo "$line" >> "$m3u_output_file"
                converted_lines=$((converted_lines + 1))
            fi
        done < "$output_file"

        # 统计转换结果
        m3u_total_lines=$(wc -l < "$m3u_output_file" 2>/dev/null || echo "0")
        echo "✓ M3U转换完成: $m3u_output_file"
        echo "  转换频道数: $converted_lines"
        echo "  M3U总行数: $m3u_total_lines"

    else
        echo "⚠ 排序文件不存在，跳过M3U转换: $output_file"
    fi

else
    echo "⚠ 处理失败: $merged_file"
fi

echo ""
echo "处理结果："
echo "  原始文件目录: $INPUT_DIR/"
echo "  排序后文件目录: $OUTPUT_DIR/"
echo "  合并排序文件: $output_file"
if [ -f "$m3u_output_file" ]; then
    echo "  M3U播放列表: $m3u_output_file"
fi