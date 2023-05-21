use models::error;
use chrono::Duration;

// à ameliorer
pub fn parse(str: &String) -> Result<chrono::Duration, error::Interaction> {
    let a = std::time::Duration::from_secs(str.parse::<u64>()?);
    Ok(chrono::Duration::from_std(a)?)
}

pub trait DisplayTimestamp {
    fn display_timestamp(&self) -> String;
}

impl DisplayTimestamp for Duration {
    fn display_timestamp(&self) -> String {
        let minutes = self.num_minutes();
        let seconds = self.num_seconds();
        let millis = self.num_milliseconds();
        format!("{minutes:0>2}:{seconds:0>2}.{millis:0>2}")
    }
}