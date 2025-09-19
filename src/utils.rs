use chrono::Local;

pub fn format_date(date: &chrono::DateTime<Local>) -> String {
    date.format("%Y/%m/%d %H:%M").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_date_today() {
        let now = Local::now();
        let formatted = format_date(&now);
        assert!(formatted.contains('/'));
        assert!(formatted.contains(':'));
        assert_eq!(formatted.len(), 16); // YYYY/MM/DD HH:MM
    }

    #[test]
    fn test_format_date_this_week() {
        let date = Local::now() - chrono::Duration::days(3);
        let formatted = format_date(&date);
        assert!(formatted.contains('/'));
        assert!(formatted.contains(':'));
        assert_eq!(formatted.len(), 16); // YYYY/MM/DD HH:MM
    }

    #[test]
    fn test_format_date_older() {
        let date = Local::now() - chrono::Duration::days(30);
        let formatted = format_date(&date);
        assert!(formatted.contains('/'));
        assert!(formatted.contains(':'));
        assert_eq!(formatted.len(), 16); // YYYY/MM/DD HH:MM
    }
}