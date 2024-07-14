use chrono::{DateTime, Utc};

#[cfg(not(test))]
/// 現在のUTC時間を取得する。
pub fn now() -> DateTime<Utc> {
    Utc::now()
}

/// テスト時に利用するモック時間を取得する。
#[cfg(test)]
pub mod mock_datetime {
    use std::cell::RefCell;

    use super::DateTime;
    use super::Utc;

    thread_local! {
        static MOCK_TIME: RefCell<Option<DateTime<Utc>>> = RefCell::new(None);
    }

    /// モック時間を取得する。
    pub fn now() -> DateTime<Utc> {
        MOCK_TIME.with(|cell| cell.borrow().as_ref().cloned().unwrap_or_else(Utc::now))
    }

    /// モック時間を設定する。
    pub fn set_mock_time(time: DateTime<Utc>) {
        MOCK_TIME.with(|cell| *cell.borrow_mut() = Some(time));
    }

    // 設定したモック時間をクリアする。
    pub fn clear_mock_time() {
        MOCK_TIME.with(|cell| *cell.borrow_mut() = None);
    }
}

#[cfg(test)]
pub use mock_datetime::now;

#[cfg(test)]
mod tests {
    use chrono::{DateTime, SecondsFormat, Utc};

    use super::mock_datetime;

    /// 何も設定しない場合は、現在時間が取得できることを確認する。
    ///
    ///  - 現在時刻での比較を行なっているため、ミリ秒単位まで比較するとテストが失敗する可能性があり、秒単位で比較している。
    #[test]
    fn test_now() {
        assert_eq!(
            mock_datetime::now().to_rfc3339_opts(SecondsFormat::Secs, true),
            Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
        );
    }

    /// モック時間を設定した時に、その時間が取得できることを確認する。
    #[test]
    fn test_now_specific_datetime() {
        let datetime = String::from("2024-01-01T00:00:00+00:00");
        mock_datetime::set_mock_time(
            DateTime::parse_from_rfc3339(datetime.as_str())
                .unwrap()
                .to_utc(),
        );

        assert_eq!(mock_datetime::now().to_rfc3339(), datetime);
    }

    /// モック時間をリセットした時に、現在時間が取得できることを確認する。
    #[test]
    fn test_now_after_clear_mock_time() {
        let datetime = String::from("2024-01-01T00:00:00+00:00");
        mock_datetime::set_mock_time(
            DateTime::parse_from_rfc3339(datetime.as_str())
                .unwrap()
                .to_utc(),
        );
        mock_datetime::clear_mock_time();

        assert_eq!(
            mock_datetime::now().to_rfc3339_opts(SecondsFormat::Secs, true),
            Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
        );
    }
}
