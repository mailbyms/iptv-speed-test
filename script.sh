#!/bin/bash

# 配置
SUBSCRIBE_FILE="subscribe.txt"
DOWNLOAD_DIR="downloads"
OUTPUT_DIR="filtered"
CHECK_DIR="checked"

rm -fr "$CHECK_DIR"
mkdir -p "$CHECK_DIR"
rm -fr "$DOWNLOAD_DIR" "$OUTPUT_DIR" 

echo "开始处理订阅源下载..."

# 转换m3u文件为文本格式
convert_m3u_to_text() {
    local m3u_file="$1"
    local txt_file="$2"
    local txt_path="$DOWNLOAD_DIR/$txt_file"
    local counter=1
    local channel_name=""

    # 创建输出文件
    {
        echo "📺M3U转换频道,#genre#"

        # 读取m3u文件
        while IFS= read -r line; do
            # 跳过以#开头的行（除非是EXTINF）
            if [[ "$line" =~ ^# ]] && [[ ! "$line" =~ ^#EXTINF ]]; then
                continue
            fi

            # 处理EXTINF行（频道信息）
            if [[ "$line" =~ ^#EXTINF ]]; then
                # 提取频道名称 - 多种格式支持
                # 格式1: #EXTINF:-1,频道名称
                # 格式2: #EXTINF:-1 tvg-id="xxx",频道名称
                # 格式3: #EXTINF:-1 group-title="xxx",频道名称
                if [[ "$line" =~ ,(.+)$ ]]; then
                    channel_name="${BASH_REMATCH[1]}"
                    # 去掉前后空格和特殊字符
                    channel_name=$(echo "$channel_name" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')
                fi
                # 读取下一行作为URL
                read -r url_line
                if [[ "$url_line" =~ ^http ]]; then
                    # 如果没有提取到频道名称，使用默认值
                    if [ -z "$channel_name" ]; then
                        channel_name="频道$counter"
                    fi
                    echo "$channel_name,$url_line"
                    counter=$((counter + 1))
                    channel_name=""
                fi
            # 处理直接的URL行（没有EXTINF的情况）
            elif [[ "$line" =~ ^http ]]; then
                echo "频道$counter,$line"
                counter=$((counter + 1))
            fi
        done < "$m3u_file"
    } > "$txt_path"

    # 删除原始m3u文件
    rm -f "$m3u_file"
    echo "  ✓ 转换完成: $txt_file ($((counter-1))个频道)"
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
echo "开始过滤下载的文件..."

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

        # 步骤1: 去掉包含 swf 或 drm 的行（不区分大小写）
        sed -i '/swf\|drm/Id' "$temp_file"

        # 步骤2: 保留包含 http 或 https 的行
        sed -i '/https\?:\/\//!d' "$temp_file"

        # 步骤3: 去掉包含IP地址URL的行
        sed -i '/,https\?:\/\/\b\([0-9]\{1,3\}\.\)\{3\}[0-9]\{1,3\}\b\([:0-9]\+\)\?\//d' "$temp_file"

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
echo "开始检查m3u8地址可访问性..."

# 检查m3u8地址可访问性
check_m3u8_urls() {
    local input_file="$1"
    local output_file="$2"
    local total_lines=0
    local temp_dir

    # 创建临时目录存放结果
    temp_dir=$(mktemp -d)
    local valid_file="$temp_dir/valid.txt"

    # 统计总行数 - 统计所有包含http的行
    total_lines=$(grep -c "http" "$input_file" 2>/dev/null || echo "0")

    if [ $total_lines -eq 0 ]; then
        echo "  文件中没有找到http地址"
        rm -rf "$temp_dir"
        return
    fi

    echo "  检查文件: $(basename "$input_file")"
    echo "  总共 $total_lines 个地址需要检查"

    # 定义检查URL的函数
    check_url() {
        local line="$1"
        local valid_file="$2"

        # 使用更robust的方法解析行，处理包含引号的频道名
        # 找到最后一个逗号，它前面是频道名，后面是URL
        local url="${line##*,}"
        local name="${line%,*}"

        # 去除频道名前后的引号和空白
        name=$(echo "$name" | sed "s/^[\"']\|[\"']$//g; s/^[[:space:]]*//;s/[[:space:]]*$//")

        # 使用HEAD请求检查URL可访问性
        if curl -s --connect-timeout 5 --max-time 10 -I -f "$url" > /dev/null 2>&1; then
            echo "$name,$url" >> "$valid_file"
        fi
    }
    export -f check_url

    # 使用parallel进行并发处理，显示进度
    grep "[,'']http" "$input_file" | parallel --progress -j 10 'check_url {} "'"$valid_file"'"'

    # 统计结果 - 在删除临时文件之前
    local valid=0
    if [ -f "$valid_file" ]; then
        valid=$(wc -l < "$valid_file" 2>/dev/null || echo "0")
    fi
    local invalid=$((total_lines - valid))
    local efficiency

    if [ $total_lines -gt 0 ]; then
        efficiency=$(echo "scale=1; $valid * 100 / $total_lines" | bc -l 2>/dev/null || echo "N/A")
    else
        efficiency="N/A"
    fi

    # 创建输出文件
    {
        echo "📺可访问地址,#genre#"
        if [ -f "$valid_file" ]; then
            cat "$valid_file"
        fi
    } > "$output_file"

    echo ""
    echo "  检查完成！"
    echo "  可访问: $valid 个"
    echo "  不可访问: $invalid 个"
    echo "  有效率: $efficiency%"

    # 清理临时文件
    rm -rf "$temp_dir"
}

# 为所有文件创建检查目录
CHECK_DIR="checked"
rm -fr "$CHECK_DIR"
mkdir -p "$CHECK_DIR"

# 并发检查所有文件中的m3u8地址
for file in "$OUTPUT_DIR"/*.txt; do
    if [ -f "$file" ]; then
        filename=$(basename "$file")
        check_m3u8_urls "$file" "$CHECK_DIR/$filename"
    fi
done

echo ""
echo "所有检查完成！"
echo ""
echo "处理结果："
echo "  原始文件保存在: $DOWNLOAD_DIR/"
echo "  过滤后文件保存在: $OUTPUT_DIR/"
echo "  检查后文件保存在: $CHECK_DIR/"
