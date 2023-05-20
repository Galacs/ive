use models::error;

// à ameliorer
pub fn parse(str: &String) -> Result<chrono::Duration, error::Interaction> {
    let a = std::time::Duration::from_secs(str.parse::<u64>()?);
    Ok(chrono::Duration::from_std(a)?)
}