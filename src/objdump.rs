
use std::collections::HashSet;
use std::process::Command;

/// Find dependencies for the given exe/dll
///
/// ignore is a list of dll names that should not
/// be added to deps.
///
/// Returns (format, deps) format is the binary format
/// (e.g. pei-x86-64) deps is a set of dependency dlls.
///
/// Internally this calls objdump -p <file>
///
pub fn deps(file: &str, ignore: &[&str]) -> Result<(String,HashSet<String>),String> {

    let command = Command::new("objdump")
        .arg("-p")
        .arg(file)
        .output().unwrap_or_else(|e| {panic!("Failed to execute objdump: {}", e)});

    if !command.status.success() {
        return Err(format!("Error executing objdump({}): {}", 
                 command.status.code().unwrap_or(-1),
                 String::from_utf8_lossy(&command.stderr)));
    }

    let mut deps = HashSet::new();
    let out = String::from_utf8_lossy(&command.stdout);
    let mut format = String::new();
    
    for line in out.lines() {
        let xline = line.trim();
        if xline.starts_with("DLL Name:") {
            if let Some(dll) = xline.split(": ").nth(1) {
                if !ignore.contains(&dll.to_lowercase().as_ref()) {
                    deps.insert(String::from(dll));
                }
            }
        } else if xline.contains(file) {
            format = xline.split(':')
                .last().unwrap_or("").trim()
                .split_whitespace().last().unwrap_or("").to_owned();
        }
    }

    Ok((format,deps))
}

