# IPTV Speed Test - Rust 实现

基于 Rust 的 IPTV 流媒体测速工具，支持 HTTP 直连、HLS/M3U8 流和 Udpxy 代理的速率测试。

## 功能特性

- 🚀 **高性能**: 基于 Rust 和 Tokio 异步运行时
- 🌐 **多协议支持**: 支持 HTTP 直连、HLS/M3U8 流和 Udpxy 代理
- 📺 **递归解析**: 支持 M3U8 主播放列表递归解析，自动选择最佳码率
- ⚡ **并发测试**: 支持多片段并发下载测试
- 📊 **详细指标**: 提供延迟、速度、文件大小等详细信息
- 🛡️ **错误处理**: 完善的错误处理和超时控制
- 🔧 **可配置**: 支持超时时间、并发数量等参数配置
- 📝 **批量测试**: 支持从文件读取 URL 列表进行批量测试

## 构建和运行

### 前置要求

- Rust 1.70+
- Cargo

### 构建项目

```bash
cd iptv-speed-test
cargo build --release
```

### 运行示例

```bash
# 基本用法
cargo run -- "http://example.com/stream.m3u8"

# 指定超时时间和并发数
cargo run -- "http://example.com/stream.m3u8" --timeout 15 --concurrent 3

# 详细输出模式
cargo run -- "http://example.com/stream.m3u8" --verbose

# 批量测试
./examples/batch_test.sh examples/test_urls.txt
```

## 使用说明

### 命令行参数

```bash
iptv-speed-test [OPTIONS] <URL>

Arguments:
  <URL>                   要测试的HTTP URL [required]

Options:
  -t, --timeout <TIMEOUT> 超时时间（秒）[default: 10]
  -c, --concurrent <COUNT> 并发数量 [default: 5]
  -v, --verbose           详细输出
  -h, --help              Print help
  -V, --version           Print version
```

### 测试示例

#### 1. 测试 HTTP 直连流
```bash
cargo run -- "http://example.com/video.mp4"
```

#### 2. 测试 HLS/M3U8 流
```bash
cargo run -- "http://example.com/live.m3u8"
```

#### 3. 测试 Udpxy 代理流
```bash
cargo run -- "http://example.com:8800/rtp/239.1.1.1:1234"
```

#### 4. 测试 M3U8 主播放列表（递归解析）
```bash
cargo run -- "http://example.com/master.m3u8" --verbose
```

#### 5. 高并发测试
```bash
cargo run -- "http://example.com/stream.m3u8" --concurrent 10 --timeout 20
```

#### 6. 详细模式测试
```bash
cargo run -- "http://example.com/stream.m3u8" --verbose
```

### 批量测试

#### 1. 准备 URL 文件
创建 `test_urls.txt` 文件，每行一个 URL：
```
# 测试 URL 列表
# HTTP 直连测试
https://example.com/video.mp4

# M3U8/HLS 测试
http://example.com/live.m3u8

# Udpxy 链接测试
http://example.com:8800/rtp/239.1.1.1:1234
```

#### 2. 运行批量测试
```bash
# 使用内置脚本
./examples/batch_test.sh test_urls.txt

# 或手动设置
chmod +x examples/batch_test.sh
./examples/batch_test.sh examples/test_urls.txt
```

## 输出结果

### 成功测试输出示例

#### HTTP 直连测试
```
开始测试 URL: https://example.com/video.mp4

=== 测试结果 ===
URL: https://example.com/video.mp4
状态: 成功
延迟: 177.00 ms
下载速度: 1.50 MB/s
下载大小: 0.95 MB
测试时长: 0.63 秒
协议类型: HTTP直连
详细信息: 直接下载测速完成
```

#### HLS/M3U8 测试
```
开始测试 URL: http://example.com/live.m3u8

=== 测试结果 ===
URL: http://example.com/live.m3u8
状态: 成功
延迟: 3310.00 ms
下载速度: 2.78 MB/s
下载大小: 9.20 MB
测试时长: 3.88 秒
协议类型: HLS/M3U8
详细信息: HLS流测试 - 总片段: 3, 成功: 3, 平均速度: 2.78 MB/s
```

#### Udpxy 代理测试
```
开始测试 URL: http://example.com:8800/rtp/239.1.1.1:1234

=== 测试结果 ===
URL: http://example.com:8800/rtp/239.1.1.1:1234
状态: 成功
延迟: -1.00 ms
下载速度: 1.00 MB/s
下载大小: 3.08 MB
测试时长: 3.07 秒
协议类型: Udpxy代理
详细信息: Udpxy UDP多播流代理测试完成
```

### 失败测试输出示例
```
开始测试 URL: http://example.com/invalid.m3u8

=== 测试结果 ===
URL: http://example.com/invalid.m3u8
状态: 失败
延迟: -1.00 ms
下载速度: 0.00 MB/s
下载大小: 0.00 MB
测试时长: 10.00 秒
协议类型: HLS/M3U8
详细信息: HLS测试超时
```

## 技术架构

### 核心模块

1. **SpeedTester** (`src/speed_test.rs`)
   - HTTP 客户端配置和管理
   - URL 类型检测（HTTP 直连、M3U8 流、Udpxy 代理）
   - 并发控制和超时管理
   - 测试结果计算和统计
   - Udpxy 代理流处理

2. **M3u8Parser** (`src/m3u8_parser.rs`)
   - M3U8 文件解析（主播放列表和媒体播放列表）
   - 递归播放列表解析（队列式迭代，避免 async 递归）
   - 最佳码率选择
   - 片段 URL 解析和路径拼接
   - 循环访问检测

3. **Main** (`src/main.rs`)
   - 命令行参数解析（clap）
   - 程序入口和流程控制
   - 结果格式化输出

4. **Batch Test** (`examples/batch_test.sh`)
   - 批量 URL 测试脚本
   - 空行和注释过滤
   - 超时控制和错误处理

### 算法实现

#### HTTP 直连测试
1. 发送 HTTP GET 请求
2. 测量连接建立时间（延迟）
3. 流式下载内容并计算速度
4. 记录下载大小和总时间

#### HLS/M3U8 测试
1. 检测 URL 类型（Content-Type 或文件扩展名）
2. 下载并解析 M3U8 播放列表
3. **递归解析**：如果是主播放列表，自动选择最高码率子播放列表
4. 提取媒体片段 URL（最多 5 个）
5. 并发下载测试片段
6. 计算平均速度和成功率

#### Udpxy 代理测试
1. 检测 Udpxy URL 模式（/rtp/ + 多播地址）
2. 使用 GET 请求启动流转发
3. 限时读取流数据（3秒）
4. 计算实际传输速度

### 错误处理策略

- **超时处理**: 使用 `tokio::time::timeout` 控制最大等待时间
- **HTTP 错误**: 检查状态码，处理 4xx/5xx 错误
- **网络错误**: 捕获并记录连接失败、DNS 解析失败等
- **解析错误**: 处理无效的 M3U8 格式和 URL 格式

## 性能特点

### 并发优化
- 使用 Tokio 异步运行时实现高并发
- 支持配置并发数量，平衡速度和稳定性
- HLS 测试时并发下载多个片段

### 内存优化
- 流式下载，避免大文件占用过多内存
- 及时释放网络资源和缓冲区
- 使用 `Vec<u8>` 高效处理二进制数据

### 网络优化
- 连接复用减少握手开销
- 支持自定义超时时间
- 忽略 SSL 证书验证（用于测试环境）

## 扩展开发

### 添加新的协议支持
可以在 `SpeedTester` 中添加新的协议检测和处理逻辑：

```rust
async fn is_rtsp_url(&self, url: &str) -> Result<bool> {
    // RTSP 协议检测逻辑
}

async fn test_rtsp_url(&self, url: &str) -> Result<SpeedTestResult> {
    // RTSP 测试逻辑
}
```

### 已实现功能
1. ✅ **递归 M3U8 解析**: 支持主播放列表自动选择最佳码率
2. ✅ **Udpxy 代理支持**: 支持 UDP 多播流的 HTTP 代理测试
3. ✅ **批量测试**: 支持从文件读取 URL 列表进行批量测试
4. ✅ **增强错误处理**: 完善的超时控制和错误恢复机制

### 未来增强功能建议
1. **分辨率检测**: 集成 FFmpeg 进行视频分辨率检测
2. **缓存机制**: 实现测速结果缓存避免重复测试
3. **结果导出**: 支持将结果导出为 JSON/CSV 格式
4. **代理支持**: 添加 HTTP/SOCKS 代理支持
5. **RTSP/RTMP 支持**: 扩展对其他流媒体协议的支持
6. **历史记录**: 保存测速历史和趋势分析

## 许可证

MIT License

## 贡献

欢迎提交 Issue 和 Pull Request！