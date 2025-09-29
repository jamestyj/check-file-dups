pub fn format_number(n: usize) -> String {
    let s = n.to_string();
    let mut result = String::new();
    
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    
    result.chars().rev().collect()
}

pub fn format_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    const THRESHOLD: u64 = 1024;
    
    if bytes == 0 {
        return "0 B".to_string();
    }
    
    let mut size = bytes as f64;
    let mut unit_index = 0;
    
    while size >= THRESHOLD as f64 && unit_index < UNITS.len() - 1 {
        size /= THRESHOLD as f64;
        unit_index += 1;
    }
    
    if unit_index == 0 {
        format!("{} {}", bytes, UNITS[unit_index])
    } else {
        format!("{:.1} {}", size, UNITS[unit_index])
    }
}

pub fn format_human_elapsed(elapsed: std::time::Duration) -> String {
    let elapsed_secs = elapsed.as_secs();
    let elapsed_subsec_millis = elapsed.subsec_millis();
    if elapsed_secs >= 3600 {
        // Format as h:mm:ss
        let hours = elapsed_secs / 3600;
        let minutes = (elapsed_secs % 3600) / 60;
        let seconds = elapsed_secs % 60;
        format!("{hours}:{minutes:02}:{seconds:02}.{elapsed_subsec_millis:03} (h:mm:ss.mmm)")
    } else if elapsed_secs >= 60 {
        // Format as m:ss
        let minutes = elapsed_secs / 60;
        let seconds = elapsed_secs % 60;
        format!("{minutes}:{seconds:02}.{elapsed_subsec_millis:03} (m:ss.mmm)")
    } else {
        // Format as s.mmm
        format!("{}.{:03} seconds", elapsed_secs, elapsed_subsec_millis)
    }
}

pub struct FileInfo {
    pub path: std::path::PathBuf,
    pub size: u64,
    pub hash: String,
}
