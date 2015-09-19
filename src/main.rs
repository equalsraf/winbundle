
extern crate clap;

use clap::{App,Arg,SubCommand,AppSettings};
use std::collections::HashSet;
use std::path::{Path,PathBuf};
use std::fs::{copy,create_dir_all,metadata};
use std::env;

mod objdump;

// list of system dlls (lowercase)
const SYSLIBS: &'static [&'static str] = &[
    "advapi32.dll",
    "kernel32.dll",
    "msvcrt.dll",
    "shell32.dll",
    "userenv.dll",
    "ws2_32.dll",
];

/// Check if a DLL file exists and conforms to the given format
fn dll_exists(path: &PathBuf, dllformat: &str) -> bool {
    if let Ok(meta) = metadata(&path) {
        if meta.is_file() {
            match objdump::deps(&path.to_string_lossy(), SYSLIBS) {
                Ok((ref fmt,_)) if fmt == dllformat  => true,
                Ok((ref fmt,_)) => {
                    println!("Skipping {}({}!={})", &path.to_string_lossy(), fmt, dllformat);
                    false
                },
                Err(err) => {
                    println!("{}", err);
                    false
                },
            }
        } else {
            false
        }
    } else {
        false
    }
}

/// Find the absolute path for dll
///
/// If sysroot is not an empty string, it is used as a root path to find
/// the DLLs (e.g. sysroot/bin and sysroot/lib). If sysroot is empty the
/// $PATH environment variable is used, if $PATH is not set fallback to
/// the current directory.
///
fn find_dll(dllname: &str, dllformat: &str, sysroot: &str) -> Option<PathBuf> {

    if sysroot.is_empty() {
        match env::var_os("PATH") {
            Some(env_path) => for prefix in env::split_paths(&env_path) {
                let mut path = prefix.clone();
                path.push(dllname);
                if dll_exists(&path, dllformat) {
                    return Some(path)
                }
            },
            None => {
                let path = Path::new(dllname).to_path_buf();
                if dll_exists(&path, dllformat) {
                    return Some(path)
                }
            },
        }
    } else {
        // Try sysroot, sysroot/bin, sysroot/lib
        for prefix in &["", "bin", "lib"] {
            let mut p = Path::new(sysroot).to_path_buf();
            p.push(prefix);
            p.push(dllname);
            if dll_exists(&p, dllformat) {
                return Some(p);
            }
        }
    }
    None
}

fn main() {
    let args = App::new("winbundle")
                .about("Bundle DLLs for Windows targets")
                .settings(&[AppSettings::SubcommandRequired,
                    AppSettings::VersionlessSubcommands,
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

    let mut dllfmt = String::new();
    let mut deps: HashSet<String> = HashSet::new();
    for obj in cmdargs.values_of("obj").unwrap() {
        if let Ok((format, objdeps)) = objdump::deps(obj, SYSLIBS) {
            if !dllfmt.is_empty() && format != dllfmt {
                println!("We don't support mixed binaries ({} vs {})", format, dllfmt);
                return;
            }
            dllfmt = format;
            deps = deps.union(&objdeps).cloned().collect();
        } else {
            return;
        }
    }

    if cmdname == "bundle" {
        let outpath = Path::new(cmdargs.value_of("outpath").unwrap());
        if let Err(err) = create_dir_all(outpath) {
            println!("Error creating output path({}): {}", outpath.to_string_lossy(), err);
            return;
        }

        // Copy dependencies to outpath
        for dllname in deps {
            let src = find_dll(&dllname, &dllfmt, args.value_of("sysroot").unwrap_or(""))
                            .unwrap_or_else(||{ panic!("Unable to find {}", dllname) });
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
        for dllname in deps {
            let src = find_dll(&dllname, &dllfmt, args.value_of("sysroot").unwrap_or(""))
                            .unwrap_or_else(||{ panic!("Unable to find {}", dllname) });
            println!("{}", src.to_string_lossy());
        }
    }
}
