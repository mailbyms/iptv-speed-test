use clap::Parser;
use std::time::Duration;
use anyhow::Result;

mod speed_test;
mod m3u8_parser;

use speed_test::SpeedTester;

#[derive(Parser)]
#[command(name = "iptv-speed-test")]
#[command(about = "IPTV流媒体测速工具", long_about = None)]
struct Cli {
    /// 要测试的HTTP URL
    url: String,

    /// 超时时间（秒）
    #[arg(short = 't', long, default_value = "10")]
    timeout: u64,

    /// 并发数量
    #[arg(short = 'c', long, default_value = "5")]
    concurrent: usize,

    /// 详细输出
    #[arg(short = 'v', long)]
    verbose: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let tester = SpeedTester::new(
        Duration::from_secs(cli.timeout),
        cli.concurrent,
        cli.verbose,
    );

    println!("开始测试 URL: {}", cli.url);

    match tester.test_url(&cli.url).await {
        Ok(result) => {
            println!("\n=== 测试结果 ===");
            println!("URL: {}", result.url);
            println!("状态: {}", if result.success { "成功" } else { "失败" });
            println!("延迟: {:.2} ms", result.delay_ms);
            println!("下载速度: {:.2} MB/s", result.speed_mbps);
            println!("下载大小: {:.2} MB", result.size_mb);
            println!("测试时长: {:.2} 秒", result.duration_secs);
            println!("协议类型: {}", result.protocol_type);

            if let Some(details) = result.details {
                println!("详细信息: {}", details);
            }
        }
        Err(e) => {
            eprintln!("测试失败: {}", e);
        }
    }

    Ok(())
}