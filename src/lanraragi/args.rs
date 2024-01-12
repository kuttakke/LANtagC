use clap::Parser;
use std::sync::OnceLock;

/// 为Lanraragi的作品增添中文标签，仅限无标签作品
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None, styles=get_styles())]
pub struct Args {
    /// Lanraragi的URL;例 192.168.0.1:3000
    #[arg(short, long)]
    pub endpoint: String,

    /// Lanraragi的API key
    #[arg(short, long)]
    pub api_key: String,

    /// EX的Cookies;格式为：`igneous=xxx; ipb_member_id=xxx; ipb_pass_hash=xxx`
    #[arg(short, long)]
    pub cookies: String,
}

pub fn args() -> &'static Args {
    static ARGS: OnceLock<Args> = OnceLock::new();
    ARGS.get_or_init(|| {
        Args::parse()
    })
}

fn get_styles() -> clap::builder::Styles {
    clap::builder::Styles::styled()
        .usage(
            anstyle::Style::new()
                .bold()
                .underline()
                .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Yellow))),
        )
        .header(
            anstyle::Style::new()
                .bold()
                .underline()
                .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Yellow))),
        )
        .literal(
            anstyle::Style::new().fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Green))),
        )
        .invalid(
            anstyle::Style::new()
                .bold()
                .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Red))),
        )
        .error(
            anstyle::Style::new()
                .bold()
                .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Red))),
        )
        .valid(
            anstyle::Style::new()
                .bold()
                .underline()
                .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Green))),
        )
        .placeholder(
            anstyle::Style::new().fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::White))),
        )
}
