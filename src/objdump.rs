
use std::process::Command;
use std::fs::metadata;

/// Find dependencies for the given exe/dll
///
/// Returns (format, deps) format is the binary format
/// (e.g. pei-x86-64) deps is the list of dependency dlls.
///
/// Internally this calls objdump -p <file>
///
pub fn deps(file: &str) -> Result<(String,Vec<String>),String> {

    match metadata(file) {
        Err(err) => return Err(format!("Error opening {}: {}", file, err)),
        Ok(ref meta) if !meta.is_file() => return Err(format!("Not a file: {}", file)),
        _ => (),
    }

    let command = Command::new("objdump")
        .arg("-p")
        .arg(file)
        .output().unwrap_or_else(|e| {panic!("Failed to execute objdump: {}", e)});

    if !command.status.success() {
        return Err(format!("Error executing objdump({}): {}", 
                 command.status.code().unwrap_or(-1),
                 String::from_utf8_lossy(&command.stderr)));
    }

    let mut deps = Vec::new();
    let out = String::from_utf8_lossy(&command.stdout);
    let mut format = String::new();
    
    for line in out.lines() {
        let xline = line.trim();
        if xline.starts_with("DLL Name:") {
            if let Some(dll) = xline.split(": ").nth(1) {
                deps.push(String::from(dll));
            }
        } else if xline.contains(file) {
            format = xline.split(':')
                .last().unwrap_or("").trim()
                .split_whitespace().last().unwrap_or("").to_owned();
        }
    }

    Ok((format,deps))
}

