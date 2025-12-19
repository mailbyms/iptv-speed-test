#!/bin/bash

# 配置
SUBSCRIBE_FILE="subscribe.txt"
DOWNLOAD_DIR="downloads"
OUTPUT_DIR="filtered"

# 关键词白名单配置 (用空格分隔多个关键词)
# 只有包含这些关键词的行才会被保留
WHITELIST_KEYWORDS="广东 广州 深圳 香港卫视 无线新闻 明珠台 翡翠台 凤凰卫视 卡通 少儿 /rtp/"
rm -fr "$DOWNLOAD_DIR" "$OUTPUT_DIR" 

echo "开始处理订阅源下载..."

# 转换m3u文件为文本格式
convert_m3u_to_text() {
    local m3u_file="$1"
    local txt_file="$2"
    local txt_path="$DOWNLOAD_DIR/$txt_file"
    local channel_name=""
    local start_time=$(date +%s)
    local processed=0
    local total_lines=0

    # 计算文件有效链接（以http开头）行数用于进度显示
    total_lines=$(grep -c "^http" "$m3u_file" 2>/dev/null || echo "0")

    if [ "$total_lines" -eq 0 ]; then
        echo "  ⚠ 文件为空或无法读取"
        return 1
    fi

    echo "  🔄 开始转换 M3U 文件 (有效链接共 $total_lines)..."
    
    # 清理m3u文件：去掉空行、只保留#EXTINF或http开头的行
    sed -i '/^$/d' "$DOWNLOAD_DIR/$filename"  # 去掉空行
    sed -i '/^#EXTINF\|^http/!d' "$DOWNLOAD_DIR/$filename"  # 只保留#EXTINF或http开头的行
    echo "  文件清理完成，开始转换格式..."

    # 创建输出文件
    {
        # 读取m3u文件并显示进度
        local prev_line=""
        while IFS= read -r line; do

            # 每处理10行显示一次进度
            local progress_interval=10

            if [ $((processed % progress_interval)) -eq 0 ] || [ "$processed" -eq "$total_lines" ]; then
                local progress=$((processed * 100 / total_lines))

                # 简化进度条显示，使用更兼容的方式
                local bar_width=20
                local filled=$((progress * bar_width / 100))
                local bar=""
                for ((i=0; i<filled; i++)); do bar+="="; done
                for ((i=filled; i<bar_width; i++)); do bar+="."; done

                # 将进度信息输出到stderr，避免被重定向到文件，并在同一行刷新
                # 使用printf %%来转义百分号，避免格式化警告
                printf "\r  进度: [${bar}] ${progress}%% (%d/%d 行)" "${progress}" "${progress}" "${processed}" "${total_lines}" >&2

                # 如果是最后一次，添加换行符
                if [ "$processed" -eq "$total_lines" ]; then
                    echo "" >&2
                fi
            fi
          
            # 当前行是http开头的URL行时处理
            if [[ "$line" =~ ^http ]]; then
                local channel_name=""

                # 检查上一行是否为EXTINF
                if [[ "$prev_line" =~ ^#EXTINF ]]; then
                    # 从EXTINF行提取频道名称 - 多种格式支持
                    # 格式1: #EXTINF:-1,频道名称
                    # 格式2: #EXTINF:-1 tvg-id="xxx",频道名称
                    # 格式3: #EXTINF:-1 group-title="xxx",频道名称
                    if [[ "$prev_line" =~ ,(.+)$ ]]; then
                        channel_name="${BASH_REMATCH[1]}"
                        # 去掉前后空格和特殊字符
                        channel_name=$(echo "$channel_name" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')
                    fi

                    # 如果没有提取到频道名称，使用默认值
                    if [ -z "$channel_name" ]; then
                        channel_name="频道$processed"
                    fi
                else
                    # 上一行不是EXTINF，生成默认频道名称
                    channel_name="频道$processed"
                fi

                # 输出频道名称和URL
                echo "$channel_name,$line"

                # 更新已处理行数
                processed=$((processed + 1))
            fi

            # 记录当前行为上一行（为下一次循环准备）
            prev_line="$line"
        done < "$m3u_file"
    } > "$txt_path"

    # 删除原始m3u文件
    rm -f "$m3u_file"

    local end_time=$(date +%s)
    local total_time=$((end_time - start_time))
    local total_channels=$processed

    if [ "$total_time" -gt 0 ]; then
        local avg_speed=$((total_channels / total_time))
        echo "  ✓ 转换完成: $txt_file ($total_channels个频道, 耗时${total_time}秒, 平均${avg_speed}频道/秒)"
    else
        echo "  ✓ 转换完成: $txt_file ($total_channels个频道, 耗时${total_time}秒)"
    fi
}

# 创建目录
mkdir -p "$DOWNLOAD_DIR"
mkdir -p "$OUTPUT_DIR"

# 读取并处理订阅源
while IFS= read -r line; do
    # 去除首尾空白
    line=$(echo "$line" | xargs)

    # 跳过空行和注释行
    if [ -z "$line" ] || [[ "$line" =~ ^# ]]; then
        continue
    fi

    # 提取文件名
    filename=$(basename "$line")
    if [ "$filename" = "/" ] || [ -z "$filename" ]; then
        filename="subscription_${RANDOM}.txt"
    fi
    filename="${RANDOM}_$filename"

    # 下载文件
    echo "下载: $line"
    if curl -fsSL -o "$DOWNLOAD_DIR/$filename" "$line"; then
        echo "✓ 下载成功: $filename"

        # 检查是否为m3u文件，如果是则转换
        if [[ "$filename" == *".m3u" ]] || [[ "$filename" == *".m3u8" ]]; then
            echo "  检测到m3u文件，正在转换..."
            convert_m3u_to_text "$DOWNLOAD_DIR/$filename" "${filename%.*}_mod.txt"
        fi
    else
        echo "✗ 下载失败: $line"
    fi

done < "$SUBSCRIBE_FILE"

echo "处理完成！"
echo "开始过滤下载的文件，过滤关键词：swf, drm，audio 只保留 http[s] 协议的..."
echo "白名单关键词: $WHITELIST_KEYWORDS"

# 处理下载目录中的所有txt文件
for file in "$DOWNLOAD_DIR"/*.txt; do
    if [ -f "$file" ]; then
        filename=$(basename "$file")
        echo "处理文件: $filename"

        # 创建临时文件
        temp_file="${file}.tmp"

        # 使用sed和awk组合快速过滤
        # 先统计原始行数
        original_lines=$(wc -l < "$file" 2>/dev/null || echo "0")

        # 复制到临时文件进行处理
        cp "$file" "$temp_file"

        # 步骤1: 去掉包含 swf 或 drm 或 audio 的行（不区分大小写）
        sed -i '/swf\|drm\|audio/Id' "$temp_file"

        # 步骤2: 保留包含 http 或 https 的行
        sed -i '/https\?:\/\//!d' "$temp_file"

        # 步骤3: 去掉包含非ASCII字符的URL行
        LC_ALL=C sed -i '/https\?:\/\/.*[\x80-\xFF]/d' "$temp_file"

        # 步骤4: 关键词白名单过滤 - 只保留包含白名单关键词的行
        if [ -n "$WHITELIST_KEYWORDS" ]; then
            echo "  应用关键词白名单过滤: $WHITELIST_KEYWORDS"

            # 构建grep模式，匹配任意一个关键词
            grep_pattern=""
            for keyword in $WHITELIST_KEYWORDS; do
                if [ -z "$grep_pattern" ]; then
                    grep_pattern="$keyword"
                else
                    grep_pattern="$grep_pattern|$keyword"
                fi
            done

            # 使用grep进行白名单过滤，不区分大小写
            grep -iE "($grep_pattern)" "$temp_file" > "${temp_file}.whitelist"
            mv "${temp_file}.whitelist" "$temp_file"
        fi

        # 将过滤后的内容写入输出目录
        if [ -f "$temp_file" ]; then
            # 统计过滤后的行数
            filtered_lines=$(wc -l < "$temp_file" 2>/dev/null || echo "0")

            # 移动到输出目录
            output_file="$OUTPUT_DIR/$filename"
            mv "$temp_file" "$output_file"
            echo "✓ 过滤完成: $original_lines 行 -> $filtered_lines 行"
            echo "  输出文件: $output_file"
        else
            echo "⚠ 没有有效内容，已跳过: $filename"
        fi
    fi
done

echo "所有文件过滤完成！"

echo ""
echo "所有检查完成！"
echo ""
echo "处理结果："
echo "  原始文件保存在: $DOWNLOAD_DIR/"
echo "  过滤后文件保存在: $OUTPUT_DIR/"
