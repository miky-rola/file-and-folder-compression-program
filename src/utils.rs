use std::time::SystemTime;

pub fn format_size(size: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if size >= GB {
        format!("{:.2} GB", size as f64 / GB as f64)
    } else if size >= MB {
        format!("{:.2} MB", size as f64 / MB as f64)
    } else if size >= KB {
        format!("{:.2} KB", size as f64 / KB as f64)
    } else {
        format!("{} B", size)
    }
}

pub fn format_time(time: SystemTime) -> String {
    time.duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| {
            let secs = d.as_secs();
            let naive = chrono::NaiveDateTime::from_timestamp_opt(secs as i64, 0)
                .unwrap_or_default();
            naive.format("%Y-%m-%d %H:%M:%S").to_string()
        })
        .unwrap_or_else(|_| String::from("Unknown"))
}
