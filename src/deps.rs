
use std::process::Command;
use std::fs::metadata;

pub fn objdump_deps_for(file: &str) -> Result<(String,Vec<String>),String> {

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

pub fn dumpbin_deps_for(file: &str) -> Result<(String,Vec<String>),String> {
    match metadata(file) {
        Err(err) => return Err(format!("Error opening {}: {}", file, err)),
        Ok(ref meta) if !meta.is_file() => return Err(format!("Not a file: {}", file)),
        _ => (),
    }

    let command = Command::new("dumpbin")
        .arg("/dependents")
        .arg(file)
        .output().unwrap_or_else(|e| {panic!("Failed to execute dumpbin: {}", e)});
    if !command.status.success() {
        return Err(format!("Error executing dumpbin({}): {}", 
                 command.status.code().unwrap_or(-1),
                 String::from_utf8_lossy(&command.stderr)));
    }

    let mut deps = Vec::new();
    let out = String::from_utf8_lossy(&command.stdout);
    let mut format = String::new();

    // dumpbin /dependents
    for line in out.lines() {
        // DLL lines start with 4 spaces, end in .dll
        if line.starts_with("    ") {
            let xline = line.trim();
            if xline.ends_with(".dll") {
	        deps.push(String::from(xline));
            }
        }
    }

    // TODO: dumpbin /headers to extract format

    Ok((format,deps))
}

/// Find dependencies for the given exe/dll
///
/// Returns (format, deps) format is the binary format
/// (e.g. pei-x86-64) deps is the list of dependency dlls.
///
/// Internally this calls either objdump or dumpbin to extract this
/// information (in this order).
///
/// The dumpbin backend currently does not return a **format** (TODO).
///
pub fn deps_for(file: &str) -> Result<(String,Vec<String>),String> {
    match objdump_deps_for(file) {
        Ok(val) => Ok(val),
        Err(_) => match dumpbin_deps_for(file) {
            Ok(val) => Ok(val),
            Err(err) => Err(err),
        }
    }
}

