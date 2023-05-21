use models::error;
use chrono::Duration;

pub fn parse(str: &String) -> Result<chrono::Duration, error::Interaction> {
    let b: Vec<_> = str.split(":").collect();
    let c: Vec<_> = b.last().unwrap().split(|c| c == ',' || c == '.').collect();
    let mut micros: u64 = if c.len() == 2 {
        format!("{:0<-5}", c.last().unwrap_or(&"")).parse().unwrap()
    } else { 0 };
    let seconds = c.first().unwrap_or(&"").parse().ok().unwrap_or(0);
    let minutes = if b.len() == 2 {
        b.get(b.len() - 2).unwrap_or(&"").parse().ok().unwrap_or(0)
    } else { 0 };

    if c.len() == 1 { micros = 0 }
    let micros = std::time::Duration::from_micros(micros);
    let seconds = std::time::Duration::from_secs(seconds);
    let minutes = std::time::Duration::from_secs(minutes*60);
    let dur = micros + seconds + minutes;

    Ok(chrono::Duration::from_std(dur)?)
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