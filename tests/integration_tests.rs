use chrono::Local;
use rutt::Email;

#[test]
fn test_email_sorting() {
    let now = Local::now();
    let mut emails = vec![
        Email {
            _uid: 1,
            subject: "First".to_string(),
            from: "a@test.com".to_string(),
            cc: None,
            bcc: None,
            date: now - chrono::Duration::days(2),
            is_read: false,
            body: None,
        },
        Email {
            _uid: 2,
            subject: "Second".to_string(),
            from: "b@test.com".to_string(),
            cc: None,
            bcc: None,
            date: now - chrono::Duration::days(1),
            is_read: true,
            body: None,
        },
        Email {
            _uid: 3,
            subject: "Third".to_string(),
            from: "c@test.com".to_string(),
            cc: None,
            bcc: None,
            date: now,
            is_read: false,
            body: None,
        },
    ];

    emails.sort_by(|a, b| b.date.cmp(&a.date));

    assert_eq!(emails[0].subject, "Third");
    assert_eq!(emails[1].subject, "Second");
    assert_eq!(emails[2].subject, "First");
}

#[test]
fn test_email_list_creation() {
    let emails = vec![
        Email {
            _uid: 100,
            subject: "Test Email 1".to_string(),
            from: "sender1@example.com".to_string(),
            cc: None,
            bcc: None,
            date: Local::now(),
            is_read: false,
            body: None,
        },
        Email {
            _uid: 101,
            subject: "Test Email 2".to_string(),
            from: "sender2@example.com".to_string(),
            cc: None,
            bcc: None,
            date: Local::now() - chrono::Duration::hours(1),
            is_read: true,
            body: None,
        },
    ];

    assert_eq!(emails.len(), 2);
    assert_eq!(emails[0]._uid, 100);
    assert_eq!(emails[1]._uid, 101);
    assert_ne!(emails[0].is_read, emails[1].is_read);
}

#[test]
fn test_email_field_validation() {
    let email = Email {
        _uid: 999,
        subject: String::new(),
        from: String::new(),
        cc: None,
        bcc: None,
        date: Local::now(),
        is_read: false,
        body: None,
    };

    assert_eq!(email.subject, "");
    assert_eq!(email.from, "");
    assert!(!email.is_read);
}

#[test]
fn test_long_subject_handling() {
    let long_subject = "A".repeat(100);
    let email = Email {
        _uid: 1000,
        subject: long_subject.clone(),
        from: "test@test.com".to_string(),
        cc: None,
        bcc: None,
        date: Local::now(),
        is_read: false,
        body: None,
    };

    assert_eq!(email.subject.len(), 100);
    assert_eq!(email.subject, long_subject);
}
