use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct PsiLine {
    pub avg10: f64,
    pub avg60: f64,
    pub avg300: f64,
    pub total_us: u64,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct CpuPressure {
    pub some: PsiLine,
    pub full: Option<PsiLine>,
}

impl CpuPressure {
    pub fn read() -> Result<Self> {
        Self::read_from("/proc/pressure/cpu")
    }

    pub fn read_from(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let text = fs::read_to_string(path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        parse_cpu_pressure(&text)
    }
}

pub fn parse_cpu_pressure(text: &str) -> Result<CpuPressure> {
    let mut some = None;
    let mut full = None;
    for line in text.lines().map(str::trim).filter(|l| !l.is_empty()) {
        let mut parts = line.split_whitespace();
        let kind = parts.next().unwrap_or_default();
        let mut parsed = PsiLine::default();
        for part in parts {
            let Some((key, value)) = part.split_once('=') else {
                continue;
            };
            match key {
                "avg10" => parsed.avg10 = value.parse()?,
                "avg60" => parsed.avg60 = value.parse()?,
                "avg300" => parsed.avg300 = value.parse()?,
                "total" => parsed.total_us = value.parse()?,
                _ => {}
            }
        }
        match kind {
            "some" => some = Some(parsed),
            "full" => full = Some(parsed),
            _ => {}
        }
    }
    Ok(CpuPressure {
        some: some.ok_or_else(|| anyhow::anyhow!("PSI 'some' line not found"))?,
        full,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_cpu_psi() {
        let p = parse_cpu_pressure(
            "some avg10=12.50 avg60=3.00 avg300=1.00 total=42\nfull avg10=0.00 avg60=0.00 avg300=0.00 total=0\n",
        )
        .unwrap();
        assert_eq!(p.some.avg10, 12.5);
        assert_eq!(p.some.total_us, 42);
    }
}

/// Convenience wrapper used by experiment runners.
pub fn read_cpu_pressure() -> Result<CpuPressure> {
    CpuPressure::read()
}
