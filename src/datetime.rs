use chrono::{DateTime, Utc};

#[cfg(not(test))]
pub fn now() -> DateTime<Utc> {
    Utc::now()
}

#[cfg(test)]
pub mod mock_datetime {
    use std::cell::RefCell;

    use super::DateTime;
    use super::Utc;

    thread_local! {
        static MOCK_TIME: RefCell<Option<DateTime<Utc>>> = RefCell::new(None);
    }

    pub fn now() -> DateTime<Utc> {
        MOCK_TIME.with(|cell| cell.borrow().as_ref().cloned().unwrap_or_else(Utc::now))
    }

    pub fn set_mock_time(time: DateTime<Utc>) {
        MOCK_TIME.with(|cell| *cell.borrow_mut() = Some(time));
    }

    pub fn clear_mock_time() {
        MOCK_TIME.with(|cell| *cell.borrow_mut() = None);
    }
}

#[cfg(test)]
pub use mock_datetime::now;

#[cfg(test)]
mod tests {
    use chrono::DateTime;

    use super::mock_datetime;

    #[test]
    fn now_ok() {
        let datetime = String::from("2024-01-01T00:00:00+00:00");
        mock_datetime::set_mock_time(
            DateTime::parse_from_rfc3339(datetime.as_str())
                .unwrap()
                .to_utc(),
        );

        assert_eq!(mock_datetime::now().to_rfc3339(), datetime);
    }
}
