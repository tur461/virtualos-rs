use anyhow::{Context, bail};

pub fn parse_memory(s: &str) -> Result<u64, anyhow::Error> {
    let s = s.trim().to_lowercase();
    if s.is_empty() {
        bail!("empty memory string");
    }
    let (num_str, suffix) = if let Some(pos) = s.find(|c: char| !c.is_ascii_digit() && c != '.') {
        (&s[..pos], &s[pos..])
    } else {
        (s.as_str(), "")
    };
    let num: f64 = num_str.parse().context("invalid memory value")?;
    let factor = match suffix {
        "k" | "kb" => 1024.0,
        "m" | "mb" => 1024.0 * 1024.0,
        "g" | "gb" => 1024.0 * 1024.0 * 1024.0,
        "" => 1.0,
        _ => bail!("unknown suffix: {}", suffix),
    };
    Ok((num * factor) as u64)
}
