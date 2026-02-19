use chrono::{DateTime, Utc};

pub fn human_bytes(b: i64) -> String {
    if b == 0 {
        return "0 B".to_string();
    }
    const UNIT: i64 = 1024;
    if b < UNIT {
        return format!("{} B", b);
    }
    let suffixes = ["KB", "MB", "GB", "TB"];
    let mut div = UNIT;
    let mut exp = 0;
    let mut n = b / UNIT;
    while n >= UNIT && exp < suffixes.len() - 1 {
        div *= UNIT;
        exp += 1;
        n /= UNIT;
    }
    format!("{:.1} {}", b as f64 / div as f64, suffixes[exp])
}

pub fn human_time(t: Option<DateTime<Utc>>) -> String {
    let t = match t {
        Some(t) => t,
        None => return "never".to_string(),
    };

    let d = Utc::now() - t;
    let secs = d.num_seconds();

    if secs < 60 {
        "just now".to_string()
    } else if secs < 3600 {
        let m = d.num_minutes();
        if m == 1 {
            "1 minute ago".to_string()
        } else {
            format!("{} minutes ago", m)
        }
    } else if secs < 86400 {
        let h = d.num_hours();
        if h == 1 {
            "1 hour ago".to_string()
        } else {
            format!("{} hours ago", h)
        }
    } else if secs < 30 * 86400 {
        let days = d.num_days();
        if days == 1 {
            "1 day ago".to_string()
        } else {
            format!("{} days ago", days)
        }
    } else {
        t.format("%b %e, %Y").to_string()
    }
}

pub fn human_duration_secs(total_secs: i64) -> String {
    if total_secs < 60 {
        format!("{}s", total_secs)
    } else if total_secs < 3600 {
        format!("{}m{}s", total_secs / 60, total_secs % 60)
    } else if total_secs < 86400 {
        format!("{}h{}m", total_secs / 3600, (total_secs % 3600) / 60)
    } else {
        let days = total_secs / 86400;
        let hours = (total_secs % 86400) / 3600;
        format!("{}d{}h", days, hours)
    }
}

pub fn parse_age(start_time: &Option<String>) -> String {
    let ts = match start_time {
        Some(s) if !s.is_empty() => s,
        _ => return String::new(),
    };

    // Try parsing RFC 3339 (K8s timestamp format)
    if let Ok(dt) = DateTime::parse_from_rfc3339(ts) {
        let d = Utc::now() - dt.to_utc();
        return human_duration_secs(d.num_seconds());
    }

    String::new()
}
