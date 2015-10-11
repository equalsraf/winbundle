#[macro_use]
extern crate clap;

use clap::{App,Arg,SubCommand,AppSettings};
use std::collections::{HashMap};
use std::path::{Path,PathBuf};
use std::fs::{copy,create_dir_all,metadata};
use std::env;

mod deps;

// list of system dlls (lowercase)
const SYSLIBS: &'static [&'static str] = &[
    "advapi32.dll",
    "gdi32.dll",
    "kernel32.dll",
    "msvcrt.dll",
    "shell32.dll",
    "userenv.dll",
    "user32.dll",
    "ws2_32.dll",
];

/// Find the absolute path for dll
///
/// If sysroot is not an empty string, it is used as a root path to find
/// the DLLs (e.g. sysroot/bin and sysroot/lib). If sysroot is empty the
/// $PATH environment variable is used, if $PATH is not set fallback to
/// the current directory.
///
fn find_dll(dllname: &str, dllformat: &str, sysroot: &str) -> Option<(PathBuf,Vec<String>)> {

    if sysroot.is_empty() {
        match env::var_os("PATH") {
            Some(env_path) => for prefix in env::split_paths(&env_path) {
                let mut p = prefix.clone();
                p.push(dllname);
                if let Ok((fmt,dlldeps)) = deps::deps_for(&p.to_string_lossy()) {
                    if fmt == dllformat {
                        return Some((p,dlldeps));
                    } else {
                        println!("Skipping {}({}!={})", &p.to_string_lossy(), fmt, dllformat);
                    }
                }
            },
            None => {
                let p = Path::new(dllname).to_path_buf();
                if let Ok((fmt,dlldeps)) = deps::deps_for(&p.to_string_lossy()) {
                    if fmt == dllformat {
                        return Some((p,dlldeps));
                    } else {
                        println!("Skipping {}({}!={})", &p.to_string_lossy(), fmt, dllformat);
                    }
                }
            },
        }
    } else {
        // Try sysroot, sysroot/bin, sysroot/lib
        for prefix in &["", "bin", "lib"] {
            let mut p = Path::new(sysroot).to_path_buf();
            p.push(prefix);
            p.push(dllname);
            if let Ok((fmt,dlldeps)) = deps::deps_for(&p.to_string_lossy()) {
                if fmt == dllformat {
                    return Some((p,dlldeps));
                } else {
                    println!("Skipping {}({}!={})", &p.to_string_lossy(), fmt, dllformat);
                }
            }
        }
    }
    None
}

fn main() {
    let args = App::new("winbundle")
                .about("Bundle DLLs for Windows targets")
                .version(&crate_version!()[..])
                .version_short("v")
                .settings(&[AppSettings::SubcommandRequired,
                    AppSettings::VersionlessSubcommands,
                    AppSettings::GlobalVersion,
                    AppSettings::UnifiedHelpMessage,
                    AppSettings::SubcommandRequiredElseHelp])
                .arg(Arg::with_name("sysroot")
                     .long("sysroot")
                     .takes_value(true))
                .subcommand(SubCommand::with_name("bundle")
                    .about("Create a usable bundle")
                    .arg(Arg::with_name("outpath")
                         .index(1)
                         .required(true))
                    .arg(Arg::with_name("obj")
                         .multiple(true)
                         .required(true))
                    )
                .subcommand(SubCommand::with_name("list")
                    .about("List DLL dependencies")
                    .arg(Arg::with_name("obj")
                         .multiple(true)
                         .required(true))
                    )
                .get_matches();

    let (cmdname, cmdargs) = match args.subcommand() {
        (cmdname, Some(cmdargs)) => (cmdname,cmdargs),
        // This should not be possible, a subcommand is required
        (_,_) => panic!("Invalid subcommand"),
    };

    // Find all direct dependency names
    let mut dllfmt = String::new();
    let mut missing_dlls: Vec<String> = Vec::new();

    for obj in cmdargs.values_of("obj").unwrap() {
        match deps::deps_for(obj) {
            Ok((format, objdeps)) => {
                if !dllfmt.is_empty() && format != dllfmt {
                    println!("We don't support mixed binaries ({} vs {})", format, dllfmt);
                    return;
                }
                dllfmt = format;
                for dll_name in objdeps {
                    if !SYSLIBS.contains(&dll_name.to_lowercase().as_ref()) {
                        missing_dlls.push(dll_name.to_owned());
                    }
                }
            },
            Err(err) => {
                println!("Error: {}", err);
                return;
            },
        }
    }

    // Find paths for all dependencies
    let mut done = HashMap::new();
    while !missing_dlls.is_empty() {
        let dllname = missing_dlls.remove(0);
        if done.contains_key(&dllname) {
            continue;
        }
        let (dllpath,dlldeps) = find_dll(&dllname, &dllfmt, args.value_of("sysroot").unwrap_or(""))
                    .unwrap_or_else(||{panic!("Unable to find {}", dllname)});

        done.insert(dllname,dllpath);
        for new_dllname in dlldeps {
            if !SYSLIBS.contains(&new_dllname.to_lowercase().as_ref()) {
                missing_dlls.push(new_dllname);
            }
        }
    }

    if cmdname == "bundle" {
        let outpath = Path::new(cmdargs.value_of("outpath").unwrap());
        if let Err(err) = create_dir_all(outpath) {
            println!("Error creating output path({}): {}", outpath.to_string_lossy(), err);
            return;
        }

        // Copy dependencies to outpath
        for (_,src) in done {
            let mut dst = outpath.to_path_buf();
            dst.push(src.file_name().unwrap());
            if let Ok(_) = metadata(&dst) {
                println!("File already exists, skipping: {}", dst.to_string_lossy());
                continue;
            }
            println!("{}", src.to_string_lossy());
            copy(&src, &dst).unwrap();
        }

        // Copy files to outpath
        for obj in cmdargs.values_of("obj").unwrap() {
            let mut dst = outpath.to_path_buf();
            dst.push(Path::new(obj).file_name().unwrap());
            copy(obj, dst).unwrap();
        }
    } else {
        for (_, path) in done {
            println!("{}", path.to_string_lossy());
        }
    }
}
