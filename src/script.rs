use std::fs;
use std::io::{self, BufRead};
use std::os::unix::fs::PermissionsExt;

#[derive(Debug)]
pub struct Script {
    pub path: String,
    pub name: String,
    pub description: Option<String>,
    pub category: Option<String>,
}

pub fn extract_description(
    path: &str
) -> Result<Option<String>, io::Error> {
    let file = fs::File::open(path)?;
    let reader = io::BufReader::new(file);
    
    for line_result in reader.lines() {
        let line = line_result?;
        let trimmed = line.trim();
        
        if trimmed.is_empty() || trimmed.starts_with("#!") {
            continue;
        }
        
        let desc = if let Some(d) = trimmed.strip_prefix('#') {
            Some(d)
        } else if let Some(d) = trimmed.strip_prefix("//") {
            Some(d)
        } else if let Some(d) = trimmed.strip_prefix("--") {
            Some(d)
        } else {
            None
        };
        
        if let Some(d) = desc {
            let cleaned = d.trim().to_string();
            if !cleaned.is_empty() {
                return Ok(Some(cleaned));
            }
            continue;
        }
        
        break;
    }
    
    Ok(None)
}
