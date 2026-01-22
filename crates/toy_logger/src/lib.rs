use env_logger::Builder;
use env_logger::fmt::style::{AnsiColor, Color, Style};
use log::{Level, LevelFilter};
use std::io::Write;

/// Initialize a stdout logger with colored, timestamped output
pub fn init(level: LevelFilter) {
    Builder::new()
        .filter_level(log::LevelFilter::Warn)
        .filter_module("toy_compiler", level)
        .format_timestamp_millis()
        .format(|buf, record| {
            let timestamp_style = Style::new()
                .dimmed()
                .fg_color(Some(Color::Ansi(AnsiColor::BrightBlack)));

            let (level_name, level_fg, level_bg) = match record.level() {
                Level::Trace => ("TRACE", AnsiColor::BrightBlack, None),
                Level::Debug => ("DEBUG", AnsiColor::Green, None),
                Level::Info => ("INFO ", AnsiColor::Blue, None),
                Level::Warn => ("WARN ", AnsiColor::Yellow, None),
                Level::Error => ("ERROR", AnsiColor::BrightWhite, Some(AnsiColor::BrightRed)),
            };
            let level_style = Style::new()
                .bold()
                .fg_color(Some(Color::Ansi(level_fg)))
                .bg_color(level_bg.map(Color::Ansi));

            let timestamp = buf.timestamp();
            write!(buf, "{}", timestamp_style.render())?;
            write!(buf, "{}", timestamp)?;
            write!(buf, "{}", timestamp_style.render_reset())?;

            write!(buf, " {}", level_style.render())?;
            write!(buf, "{}", level_name)?;
            write!(buf, "{}", level_style.render_reset())?;

            writeln!(buf, " {}", record.args())
        })
        .init();
}
