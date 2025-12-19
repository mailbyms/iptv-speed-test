#!/bin/bash
#set -x 

# 定义可执行文件、源目录和目标目录的路径
EXE_PATH="./target/release/iptv-speed-test"
FILTERED_DIR="filtered"
CHECKED_DIR="checked"

# 检查可执行文件是否存在
if [ ! -f "$EXE_PATH" ]; then
    echo "错误: 找不到测速程序 '$EXE_PATH'."
    echo "请先运行 'cargo build --release' 来编译项目."
    exit 1
fi

# 检查源目录是否存在
if [ ! -d "$FILTERED_DIR" ]; then
    echo "错误: 找不到源目录 '$FILTERED_DIR'."
    exit 1
fi

# 重新创建目标目录
rm -fr "$CHECKED_DIR" && mkdir -p "$CHECKED_DIR"

# 设置最大并发任务数量
MAX_JOBS=20

echo "开始进行速度测试 (并发数: $MAX_JOBS)..."

for file_path in $(ls -1 "$FILTERED_DIR"); do
    file_path="$FILTERED_DIR/$file_path"
    echo "$file_path"
    # 确保处理的是文件而不是目录
    if [ -f "$file_path" ]; then
        filename=$(basename "$file_path")
        output_path="$CHECKED_DIR/$filename"

        # 在每次运行时清空旧的输出文件
        > "$output_path"

        echo "---"
        echo "正在处理文件: $filename"

        # 计算总URL数量
        total_urls=$(wc -l < "$file_path")
        launched_tasks=0

        # 逐行读取文件
        while IFS= read -r line || [ -n "$line" ]; do
            # 当正在运行的任务达到最大数量时，等待任一任务完成
            while (( $(jobs -p | wc -l) >= MAX_JOBS )); do
                sleep 0.1
            done

            launched_tasks=$((launched_tasks + 1))
            printf "  测试进度: %d/%d\r" "$launched_tasks" "$total_urls"

            # 在后台处理当前行
            (
                if [ -z "$line" ]; then
                    exit
                fi

                url=$(echo "$line" | cut -d ',' -f 2-)

                if [ -z "$url" ]; then
                    echo "  [警告] 无法从此行提取URL: '$line' - 已跳过"
                    exit
                fi

                speed_output=$("$EXE_PATH" "$url")
                
                # 使用 grep 精确提取数字部分
                speed_value=$(echo "$speed_output" | grep -o -E '[0-9]+')

                # 检查速度值是否为纯数字且大于1000
                if [[ -n "$speed_value" ]] && [ "$speed_value" -gt 1000 ]; then
                    echo "  [保留] 速度: ${speed_output} - URL: $line"
                    echo "$line" >> "$output_path"
                fi
            ) &
        done < "$file_path"

        # 等待当前文件的所有后台任务完成
        wait

        echo "文件 '$filename' 处理完成. 满足条件的频道已保存至 '$output_path'."
    fi
done

echo "---"
echo "所有文件处理完毕。"


