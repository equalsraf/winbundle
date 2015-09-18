
extern crate clap;

use clap::{App,Arg,SubCommand,AppSettings};
use std::collections::HashSet;
use std::path::{Path,PathBuf};
use std::fs::{copy,create_dir_all,metadata};

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

/// Find the absolute path for dll
fn find_dll(dllname: &str, dllformat: &str, sysroot: &str) -> Option<PathBuf> {

    // Try sysroot and sysroot/bin
    for prefix in &["", "bin", "lib"] {
        let mut p = Path::new(sysroot).to_path_buf();
        p.push(prefix);
        p.push(dllname);
        if let Ok(meta) = metadata(&p) {
            if meta.is_file() {
                match objdump::deps(&p.to_string_lossy(), SYSLIBS) {
                    Ok((ref fmt,_)) if fmt == dllformat  => return Some(p),
                    Ok((ref fmt,_)) => println!("Skipping {}({}!={})", p.to_string_lossy(), fmt, dllformat),
                    Err(err) => println!("{}", err),
                }
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
