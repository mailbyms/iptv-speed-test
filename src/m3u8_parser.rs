use anyhow::Result;
use reqwest::Client;
use url::Url;

pub struct M3u8Parser {
    verbose: bool,
}

impl M3u8Parser {
    pub fn new(verbose: bool) -> Self {
        Self { verbose }
    }

    pub async fn parse_m3u8(&self, m3u8_url: &str, client: &Client) -> Result<Vec<String>> {
        use std::collections::VecDeque;

        let mut playlist_queue = VecDeque::new();
        playlist_queue.push_back(m3u8_url.to_string());
        let mut all_segments = Vec::new();
        let mut visited_urls = std::collections::HashSet::new();

        while let Some(current_url) = playlist_queue.pop_front() {
            // 避免循环访问同一个URL
            if visited_urls.contains(&current_url) {
                continue;
            }
            visited_urls.insert(current_url.clone());

            if self.verbose {
                println!("解析播放列表: {}", current_url);
            }

            // 获取M3U8文件内容
            let response = client
                .get(&current_url)
                .send()
                .await?;

            if !response.status().is_success() {
                if self.verbose {
                    println!("无法获取M3U8文件 {}: {}", current_url, response.status());
                }
                continue;
            }

            let content = response.text().await?;

            if self.verbose && current_url == m3u8_url {
                println!("M3U8文件内容预览:\n{}", &content[..content.len().min(500)]);
            }

            // 解析M3U8内容
            let (segments, is_master, best_playlist_url) = self.parse_m3u8_content(&content, &current_url)?;

            if is_master {
                // 主播放列表：将最佳子播放列表加入队列
                if let Some(playlist_url) = best_playlist_url {
                    if self.verbose {
                        println!("发现主播放列表，添加子播放列表到队列: {}", playlist_url);
                    }
                    playlist_queue.push_back(playlist_url);
                } else {
                    if self.verbose {
                        println!("主播放列表中未找到有效的播放流");
                    }
                }
            } else {
                // 媒体播放列表：添加片段到结果
                if self.verbose {
                    println!("发现媒体播放列表，{} 个媒体片段", segments.len());
                }
                all_segments.extend(segments);
            }
        }

        if self.verbose {
            println!("总共解析得到 {} 个媒体片段", all_segments.len());
        }

        Ok(all_segments)
    }

    fn parse_m3u8_content(&self, content: &str, base_url: &str) -> Result<(Vec<String>, bool, Option<String>)> {
        let lines: Vec<&str> = content.lines().collect();
        let mut segments = Vec::new();
        let mut is_master_playlist = false;
        let mut best_bandwidth = 0u64;
        let mut best_playlist_url: Option<String> = None;

        let base_url_obj = Url::parse(base_url)?;
        let _base_path = format!(
            "{}://{}{}",
            base_url_obj.scheme(),
            base_url_obj.host_str().unwrap_or(""),
            base_url_obj.path().rsplit('/').nth(1).unwrap_or("")
        );

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();

            // 跳过空行和注释
            if trimmed.is_empty() || trimmed.starts_with('#') {
                // 检查是否为主播放列表
                if trimmed.contains("EXT-X-STREAM-INF") {
                    is_master_playlist = true;

                    // 提取码率信息
                    if let Some(bandwidth) = self.extract_bandwidth(trimmed) {
                        if self.verbose {
                            println!("发现播放流，码率: {}", bandwidth);
                        }

                        // 查找下一行的URL
                        if let Some(next_line) = lines.get(i + 1) {
                            let next_trimmed = next_line.trim();
                            if !next_trimmed.is_empty() && !next_trimmed.starts_with('#') {
                                let playlist_url = self.resolve_url(next_trimmed, base_url);

                                
                                if bandwidth > best_bandwidth {
                                    best_bandwidth = bandwidth;
                                    best_playlist_url = Some(playlist_url);
                                }
                            }
                        }
                    }
                }
                continue;
            }

            // 媒体片段URL
            if !is_master_playlist {
                let segment_url = self.resolve_url(trimmed, base_url);
                segments.push(segment_url);
            }
        }

        Ok((segments, is_master_playlist, best_playlist_url))
    }

    fn extract_bandwidth(&self, line: &str) -> Option<u64> {
        // 查找 BANDWIDTH 参数
        let bandwidth_regex = regex::Regex::new(r"BANDWIDTH=(\d+)").ok()?;

        if let Some(captures) = bandwidth_regex.captures(line) {
            if let Some(bandwidth_str) = captures.get(1) {
                return bandwidth_str.as_str().parse().ok();
            }
        }

        None
    }

    fn resolve_url(&self, url: &str, base_url: &str) -> String {
        // 如果是完整的URL，直接返回
        if url.starts_with("http://") || url.starts_with("https://") {
            return url.to_string();
        }

        // 解析基础URL
        let base_url_obj = match Url::parse(base_url) {
            Ok(url) => url,
            Err(_) => return url.to_string(), // 如果解析失败，返回原始URL
        };

        // 处理绝对相对路径（以/开头）
        if url.starts_with('/') {
            format!(
                "{}://{}{}",
                base_url_obj.scheme(),
                base_url_obj.host_str().unwrap_or("localhost"),
                url
            )
        } else {
            // 相对路径：使用基础URL的目录路径
            let base_path = base_url_obj
                .path()
                .rsplit('/')
                .skip(1) // 跳过文件名，获取目录路径
                .collect::<Vec<&str>>()
                .iter()
                .rev()
                .copied()
                .collect::<Vec<&str>>()
                .join("/");

            // 构建完整的基础路径（scheme://host + base_path）
            let full_base_path = if base_path.is_empty() || base_path == "/" {
                format!("{}://{}", base_url_obj.scheme(), base_url_obj.host_str().unwrap_or("localhost"))
            } else {
                format!("{}://{}/{}", base_url_obj.scheme(), base_url_obj.host_str().unwrap_or("localhost"), base_path.trim_start_matches('/'))
            };

            format!("{}/{}", full_base_path.trim_end_matches('/'), url.trim_start_matches('/'))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_bandwidth() {
        let parser = M3u8Parser::new(false);

        assert_eq!(
            parser.extract_bandwidth("EXT-X-STREAM-INF:BANDWIDTH=1280000"),
            Some(1280000)
        );

        assert_eq!(
            parser.extract_bandwidth("EXT-X-STREAM-INF:BANDWIDTH=2560000,RESOLUTION=1920x1080"),
            Some(2560000)
        );

        assert_eq!(
            parser.extract_bandwidth("EXTINF:10.0,"),
            None
        );
    }

    #[test]
    fn test_resolve_url() {
        let parser = M3u8Parser::new(false);

        // 完整URL
        assert_eq!(
            parser.resolve_url("https://example.com/segment.ts", "https://base.com/playlist.m3u8"),
            "https://example.com/segment.ts"
        );

        // 相对URL
        assert_eq!(
            parser.resolve_url("segment.ts", "https://example.com/path/playlist.m3u8"),
            "https://example.com/path/segment.ts"
        );

        // 绝对相对URL
        assert_eq!(
            parser.resolve_url("/segment.ts", "https://example.com/path/playlist.m3u8"),
            "https://example.com/segment.ts"
        );
    }
}