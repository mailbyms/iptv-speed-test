use anyhow::{Result, anyhow};
use reqwest::Client;
use std::time::{Duration, Instant};
use futures::StreamExt;
use tokio::time::timeout;

use crate::m3u8_parser::M3u8Parser;

// HEAD检查结果枚举
#[derive(Debug)]
enum HeadCheckResult {
    M3U8,           // HEAD成功，识别为M3U8
    DirectUrl,      // HEAD成功，识别为直接URL
    FailedM3U8Suffix, // HEAD失败，但URL以.m3u8结尾
    FailedNonM3U8,  // HEAD失败，URL不以.m3u8结尾
}

#[derive(Debug, Clone)]
pub struct SpeedTestResult {
    pub url: String,
    pub success: bool,
    pub delay_ms: f64,
    pub speed_kbps: f64,
    pub size_mb: f64,
    pub duration_secs: f64,
    pub protocol_type: String,
    pub details: Option<String>,
}

pub struct SpeedTester {
    client: Client,
    timeout: Duration,
    #[allow(dead_code)]
    concurrent: usize,
    verbose: bool,
    m3u8_parser: M3u8Parser,
}

impl SpeedTester {
    pub fn new(timeout: Duration, concurrent: usize, verbose: bool) -> Self {
        // 连接超时设置为1秒
        let connect_timeout = Duration::from_secs(1);

        let client = Client::builder()
            .timeout(connect_timeout)  // 连接超时1秒
            .connect_timeout(connect_timeout)  // 连接建立超时1秒
            .danger_accept_invalid_certs(true)
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            timeout,
            concurrent,
            verbose,
            m3u8_parser: M3u8Parser::new(verbose),
        }
    }

    pub async fn test_url(&self, url: &str) -> Result<SpeedTestResult> {
        if self.verbose {
            println!("检测URL类型: {}", url);
        }

        // 先发送HEAD请求检查URL
        match self.head_check_url(url).await? {
            HeadCheckResult::M3U8 => {
                if self.verbose {
                    println!("HEAD请求成功，识别为M3U8格式");
                }
                self.test_m3u8_url(url).await
            }
            HeadCheckResult::DirectUrl => {
                if self.verbose {
                    println!("HEAD请求成功，识别为直接URL");
                }
                self.test_direct_url(url).await
            }
            HeadCheckResult::FailedM3U8Suffix => {
                if self.verbose {
                    println!("HEAD请求失败，但URL以.m3u8结尾，跳过测试");
                }
                Ok(SpeedTestResult {
                    url: url.to_string(),
                    success: false,
                    delay_ms: -1.0,
                    speed_kbps: 0.0,
                    size_mb: 0.0,
                    duration_secs: 0.0,
                    protocol_type: "HEAD失败".to_string(),
                    details: Some("HEAD请求失败且URL以.m3u8结尾，跳过测试".to_string()),
                })
            }
            HeadCheckResult::FailedNonM3U8 => {
                if self.verbose {
                    println!("HEAD请求失败，URL不以.m3u8结尾，继续GET测试");
                }
                self.test_direct_url(url).await
            }
        }
    }

    async fn head_check_url(&self, url: &str) -> Result<HeadCheckResult> {
        // 先发送HEAD请求
        let response = match self.client.head(url).send().await {
            Ok(resp) => {
                if self.verbose {
                    println!("HEAD请求成功");
                }
                resp
            }
            Err(e) => {
                if self.verbose {
                    println!("HEAD请求失败: {}", e);
                }

                // HEAD请求失败，检查URL后缀
                let basename = self.get_url_basename(url);
                if basename.to_lowercase().ends_with(".m3u8") {
                    return Ok(HeadCheckResult::FailedM3U8Suffix);
                } else {
                    return Ok(HeadCheckResult::FailedNonM3U8);
                }
            }
        };

        // 检查响应状态
        if !response.status().is_success() {
            if self.verbose {
                println!("HEAD请求状态码: {}", response.status());
            }

            // HEAD请求失败，检查URL后缀
            let basename = self.get_url_basename(url);
            if basename.to_lowercase().ends_with(".m3u8") {
                return Ok(HeadCheckResult::FailedM3U8Suffix);
            } else {
                return Ok(HeadCheckResult::FailedNonM3U8);
            }
        }

        // 检查Content-Type
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or("")
            .to_lowercase();

        let basename = self.get_url_basename(url);
        let is_m3u8_by_content_type = content_type.contains("mpegurl") || content_type.contains("m3u8");
        let is_m3u8_by_suffix = basename.to_lowercase().ends_with(".m3u8");

        if self.verbose {
            println!("Content-Type: {}", content_type);
            println!("URL basename: {}", basename);
            println!("M3U8 by Content-Type: {}", is_m3u8_by_content_type);
            println!("M3U8 by suffix: {}", is_m3u8_by_suffix);
        }

        if is_m3u8_by_content_type || is_m3u8_by_suffix {
            Ok(HeadCheckResult::M3U8)
        } else {
            Ok(HeadCheckResult::DirectUrl)
        }
    }

    // 获取URL的basename（去掉域名和路径，只保留文件名）
    fn get_url_basename(&self, url: &str) -> String {
        // 移除查询参数和锚点
        let clean_url = url.split('?').next().unwrap_or(url);
        let clean_url = clean_url.split('#').next().unwrap_or(clean_url);

        // 提取basename
        if let Some(last_slash) = clean_url.rfind('/') {
            clean_url[last_slash + 1..].to_string()
        } else {
            clean_url.to_string()
        }
    }

    
    async fn test_direct_url(&self, url: &str) -> Result<SpeedTestResult> {
        let start_time = Instant::now();

        if self.verbose {
            println!("执行直接下载测试...");
        }

        let result = timeout(self.timeout, self.download_and_measure(url)).await;

        let duration = start_time.elapsed();
        let duration_secs = duration.as_secs_f64();

        match result {
            Ok(Ok((delay_ms, speed_kbps, size_mb))) => {
                Ok(SpeedTestResult {
                    url: url.to_string(),
                    success: true,
                    delay_ms,
                    speed_kbps,
                    size_mb,
                    duration_secs,
                    protocol_type: "HTTP直连".to_string(),
                    details: Some("直接下载测速完成".to_string()),
                })
            }
            Ok(Err(e)) => {
                Ok(SpeedTestResult {
                    url: url.to_string(),
                    success: false,
                    delay_ms: -1.0,
                    speed_kbps: 0.0,
                    size_mb: 0.0,
                    duration_secs,
                    protocol_type: "HTTP直连".to_string(),
                    details: Some(format!("下载失败: {}", e)),
                })
            }
            Err(_) => {
                Ok(SpeedTestResult {
                    url: url.to_string(),
                    success: false,
                    delay_ms: -1.0,
                    speed_kbps: 0.0,
                    size_mb: 0.0,
                    duration_secs,
                    protocol_type: "HTTP直连".to_string(),
                    details: Some("请求超时".to_string()),
                })
            }
        }
    }

    async fn download_and_measure(&self, url: &str) -> Result<(f64, f64, f64)> {
        let start_time = Instant::now();

        // 使用1秒连接超时，但读取流使用3秒限制
        let response = self.client
            .get(url)
            .timeout(Duration::from_secs(3))  // 整体请求超时3秒（包含连接+读取）
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("HTTP错误: {}", response.status()));
        }

        // 计算连接延迟（到收到响应头的时间）
        let delay_ms = start_time.elapsed().as_millis() as f64;

        let content_length = response
            .headers()
            .get(reqwest::header::CONTENT_LENGTH)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<u64>().ok());

        let mut downloaded_bytes = 0u64;
        let mut stream = response.bytes_stream();

        // 设置3秒时间限制，专门用于流式下载（从连接成功后开始计算）
        let read_start = Instant::now();
        let stream_timeout = Duration::from_secs(3);

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            downloaded_bytes += chunk.len() as u64;

            // 检查是否达到3秒时间限制（从连接成功后开始计算）
            if read_start.elapsed() > stream_timeout {
                if self.verbose {
                    println!("连接成功后达到3秒读取时间限制，停止下载");
                }
                break;
            }
        }

        let total_time = start_time.elapsed().as_secs_f64();
        let size_mb = downloaded_bytes as f64 / (1024.0 * 1024.0);
        let speed_kbps = if total_time > 0.0 {
            (downloaded_bytes as f64 * 8.0) / total_time / 1024.0
        } else {
            0.0
        };

        if self.verbose {
            println!("下载完成: {:.2} MB, 耗时: {:.2} 秒, 速度: {:.0} kbps",
                     size_mb, total_time, speed_kbps);
            if let Some(length) = content_length {
                println!("Content-Length: {} bytes, 实际下载: {} bytes", length, downloaded_bytes);
            }
        }

        Ok((delay_ms, speed_kbps, size_mb))
    }

    async fn test_m3u8_url(&self, url: &str) -> Result<SpeedTestResult> {
        let start_time = Instant::now();

        if self.verbose {
            println!("执行M3U8/HLS流测试...");
        }

        let result = timeout(self.timeout, self.test_hls_stream(url)).await;

        let duration = start_time.elapsed();
        let duration_secs = duration.as_secs_f64();

        match result {
            Ok(Ok((delay_ms, speed_kbps, size_mb, details))) => {
                Ok(SpeedTestResult {
                    url: url.to_string(),
                    success: true,
                    delay_ms,
                    speed_kbps,
                    size_mb,
                    duration_secs,
                    protocol_type: "HLS/M3U8".to_string(),
                    details: Some(details),
                })
            }
            Ok(Err(e)) => {
                Ok(SpeedTestResult {
                    url: url.to_string(),
                    success: false,
                    delay_ms: -1.0,
                    speed_kbps: 0.0,
                    size_mb: 0.0,
                    duration_secs,
                    protocol_type: "HLS/M3U8".to_string(),
                    details: Some(format!("HLS测试失败: {}", e)),
                })
            }
            Err(_) => {
                Ok(SpeedTestResult {
                    url: url.to_string(),
                    success: false,
                    delay_ms: -1.0,
                    speed_kbps: 0.0,
                    size_mb: 0.0,
                    duration_secs,
                    protocol_type: "HLS/M3U8".to_string(),
                    details: Some("HLS测试超时".to_string()),
                })
            }
        }
    }

    async fn test_hls_stream(&self, url: &str) -> Result<(f64, f64, f64, String)> {
        // 解析M3U8文件
        let segments = self.m3u8_parser.parse_m3u8(url, &self.client).await?;

        if self.verbose {
            println!("发现 {} 个媒体片段", segments.len());
        }

        if segments.is_empty() {
            return Err(anyhow!("未找到有效的媒体片段"));
        }

        // 限制测试的片段数量（最多5个）
        let test_segments: Vec<String> = segments.into_iter().take(5).collect();

        let start_time = Instant::now();

        // 并发下载测试片段
        let tasks: Vec<_> = test_segments
            .iter()
            .map(|segment_url| self.download_segment_speed(segment_url))
            .collect();

        let results = futures::future::join_all(tasks).await;

        let mut total_size = 0u64;
        let mut successful_downloads = 0usize;

        for result in results {
            match result {
                Ok(size) => {
                    total_size += size;
                    successful_downloads += 1;
                }
                Err(e) => {
                    if self.verbose {
                        println!("片段下载失败: {}", e);
                    }
                }
            }
        }

        let total_time = start_time.elapsed().as_secs_f64();
        let delay_ms = start_time.elapsed().as_millis() as f64;

        let size_mb = total_size as f64 / (1024.0 * 1024.0);
        let speed_kbps = if total_time > 0.0 {
            (total_size as f64 * 8.0) / total_time / 1024.0
        } else {
            0.0
        };

        let details = format!(
            "HLS流测试 - 总片段: {}, 成功: {}, 平均速度: {:.0} kbps",
            test_segments.len(),
            successful_downloads,
            speed_kbps
        );

        Ok((delay_ms, speed_kbps, size_mb, details))
    }

    async fn download_segment_speed(&self, url: &str) -> Result<u64> {
        // 使用1秒连接超时，但读取流使用3秒限制
        let response = self.client
            .get(url)
            .timeout(Duration::from_secs(3))  // 整体请求超时3秒（包含连接+读取）
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("HTTP错误: {}", response.status()));
        }

        let mut downloaded_bytes = 0u64;
        let mut stream = response.bytes_stream();

        // 设置3秒时间限制，专门用于HLS片段流式下载（从连接成功后开始计算）
        let read_start = Instant::now();
        let stream_timeout = Duration::from_secs(3);

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            downloaded_bytes += chunk.len() as u64;

            // 检查是否达到3秒时间限制（从连接成功后开始计算）
            if read_start.elapsed() > stream_timeout {
                if self.verbose {
                    println!("HLS片段连接成功后达到3秒读取时间限制，停止下载: {}", url);
                }
                break;
            }
        }

        Ok(downloaded_bytes)
    }
}