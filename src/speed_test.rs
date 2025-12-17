use anyhow::{Result, anyhow};
use reqwest::Client;
use std::time::{Duration, Instant};
use futures::StreamExt;
use tokio::time::timeout;

use crate::m3u8_parser::M3u8Parser;

#[derive(Debug, Clone)]
pub struct SpeedTestResult {
    pub url: String,
    pub success: bool,
    pub delay_ms: f64,
    pub speed_mbps: f64,
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
        let client = Client::builder()
            .timeout(timeout)
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

        // 检查是否为 Udpxy URL
        if self.is_likely_udpxy_url(url) {
            self.test_udpxy_url(url).await
        }
        // 检查URL类型
        else if self.is_m3u8_url(url).await? {
            self.test_m3u8_url(url).await
        } else {
            self.test_direct_url(url).await
        }
    }

    async fn is_m3u8_url(&self, url: &str) -> Result<bool> {
        let response = self.client
            .head(url)
            .send()
            .await?;

        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or("")
            .to_lowercase();

        let is_m3u8 = content_type.contains("mpegurl") ||
                     content_type.contains("m3u8") ||
                     url.to_lowercase().ends_with(".m3u8");

        if self.verbose {
            println!("Content-Type: {}", content_type);
            println!("是否为M3U8: {}", is_m3u8);
        }

        Ok(is_m3u8)
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
            Ok(Ok((delay_ms, speed_mbps, size_mb))) => {
                Ok(SpeedTestResult {
                    url: url.to_string(),
                    success: true,
                    delay_ms,
                    speed_mbps,
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
                    speed_mbps: 0.0,
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
                    speed_mbps: 0.0,
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

        let response = self.client
            .get(url)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("HTTP错误: {}", response.status()));
        }

        // 计算连接延迟
        let delay_ms = start_time.elapsed().as_millis() as f64;

        let content_length = response
            .headers()
            .get(reqwest::header::CONTENT_LENGTH)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<u64>().ok());

        let mut downloaded_bytes = 0u64;
        let mut stream = response.bytes_stream();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            downloaded_bytes += chunk.len() as u64;
        }

        let total_time = start_time.elapsed().as_secs_f64();
        let size_mb = downloaded_bytes as f64 / (1024.0 * 1024.0);
        let speed_mbps = if total_time > 0.0 {
            (downloaded_bytes as f64 / total_time) / (1024.0 * 1024.0)
        } else {
            0.0
        };

        if self.verbose {
            println!("下载完成: {:.2} MB, 耗时: {:.2} 秒, 速度: {:.2} MB/s",
                     size_mb, total_time, speed_mbps);
            if let Some(length) = content_length {
                println!("Content-Length: {} bytes, 实际下载: {} bytes", length, downloaded_bytes);
            }
        }

        Ok((delay_ms, speed_mbps, size_mb))
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
            Ok(Ok((delay_ms, speed_mbps, size_mb, details))) => {
                Ok(SpeedTestResult {
                    url: url.to_string(),
                    success: true,
                    delay_ms,
                    speed_mbps,
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
                    speed_mbps: 0.0,
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
                    speed_mbps: 0.0,
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
        let speed_mbps = if total_time > 0.0 {
            (total_size as f64 / total_time) / (1024.0 * 1024.0)
        } else {
            0.0
        };

        let details = format!(
            "HLS流测试 - 总片段: {}, 成功: {}, 平均速度: {:.2} MB/s",
            test_segments.len(),
            successful_downloads,
            speed_mbps
        );

        Ok((delay_ms, speed_mbps, size_mb, details))
    }

    async fn download_segment_speed(&self, url: &str) -> Result<u64> {
        let response = self.client
            .get(url)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("HTTP错误: {}", response.status()));
        }

        let mut downloaded_bytes = 0u64;
        let mut stream = response.bytes_stream();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            downloaded_bytes += chunk.len() as u64;
        }

        Ok(downloaded_bytes)
    }

    fn is_likely_udpxy_url(&self, url: &str) -> bool {
        // Udpxy URL 的特征：
        // 1. 路径包含 /rtp/
        // 2. 后面跟着多播地址 (239.x.x.x) 和端口
        let url_lower = url.to_lowercase();
        url_lower.contains("/rtp/") &&
        url_lower.contains("239.") &&
        (url_lower.contains(":") || url_lower.matches("/").count() > 3)
    }

    async fn test_udpxy_url(&self, url: &str) -> Result<SpeedTestResult> {
        let start_time = Instant::now();

        if self.verbose {
            println!("检测到Udpxy代理URL，使用GET请求测试...");
        }

        // 直接使用GET请求测试Udpxy代理
        let result = timeout(self.timeout, self.test_http_stream(url)).await;

        let duration = start_time.elapsed();
        let duration_secs = duration.as_secs_f64();

        match result {
            Ok(Ok((delay_ms, speed_mbps, size_mb))) => {
                Ok(SpeedTestResult {
                    url: url.to_string(),
                    success: true,
                    delay_ms,
                    speed_mbps,
                    size_mb,
                    duration_secs,
                    protocol_type: "Udpxy代理".to_string(),
                    details: Some("Udpxy UDP多播流代理测试完成".to_string()),
                })
            }
            Ok(Err(e)) => {
                Ok(SpeedTestResult {
                    url: url.to_string(),
                    success: false,
                    delay_ms: -1.0,
                    speed_mbps: 0.0,
                    size_mb: 0.0,
                    duration_secs,
                    protocol_type: "Udpxy代理".to_string(),
                    details: Some(format!("Udpxy测试失败: {}", e)),
                })
            }
            Err(_) => {
                Ok(SpeedTestResult {
                    url: url.to_string(),
                    success: false,
                    delay_ms: -1.0,
                    speed_mbps: 0.0,
                    size_mb: 0.0,
                    duration_secs,
                    protocol_type: "Udpxy代理".to_string(),
                    details: Some("Udpxy测试超时".to_string()),
                })
            }
        }
    }

    async fn test_http_stream(&self, url: &str) -> Result<(f64, f64, f64)> {
        let start_time = Instant::now();
        let delay = -1.0;
        let mut total_size = 0u64;

        // Udpxy需要GET请求才能开始流转发
        let response = self.client
            .get(url)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("HTTP错误: {}", response.status()));
        }

        let mut stream = response.bytes_stream();

        // 读取数据（限制读取时间）
        let stream_timeout = Duration::from_secs(3);
        let read_start = Instant::now();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            total_size += chunk.len() as u64;

            // 限制读取时间，避免无限读取流
            if read_start.elapsed() > stream_timeout {
                break;
            }
        }

        let total_time = start_time.elapsed().as_secs_f64();
        let size_mb = total_size as f64 / (1024.0 * 1024.0);
        let speed_mbps = if total_time > 0.0 {
            (total_size as f64 / total_time) / (1024.0 * 1024.0)
        } else {
            0.0
        };

        if self.verbose {
            println!("Udpxy流读取完成: {:.2} MB, 耗时: {:.2} 秒, 速度: {:.2} MB/s",
                     size_mb, total_time, speed_mbps);
        }

        Ok((delay, speed_mbps, size_mb))
    }
}