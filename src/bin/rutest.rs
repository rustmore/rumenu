extern crate getopts;
extern crate libc;

use libc::{X_OK, W_OK, R_OK, S_ISGID, S_ISUID};

use std::fs::Metadata;
use std::os::unix::fs::MetadataExt;
use std::os::unix::fs::FileTypeExt;
use std::os::unix::raw::time_t;
use std::path::Path;
// use std::fs::PathExt;
use std::env;
use std::process::exit;
use std::fs::read_dir;
use std::fs::metadata;
use std::ffi::OsStr;
use std::io::BufRead;
use getopts::Options;


struct Config {
    a: bool,
    b: bool,
    c: bool,
    d: bool,
    e: bool,
    f: bool,
    g: bool,
    h: bool,
    l: bool,
    n: bool,
    o: bool,
    p: bool,
    q: bool,
    r: bool,
    s: bool,
    u: bool,
    w: bool,
    x: bool,
    newer: time_t,
    older: time_t,
    paths: Vec<String>,
}

fn get_config(args: Vec<String>) -> Config {
    let mut opts = Options::new();
    opts.optflag("a", "", "");
    opts.optflag("b", "", "");
    opts.optflag("c", "", "");
    opts.optflag("d", "", "");
    opts.optflag("e", "", "");
    opts.optflag("f", "", "");
    opts.optflag("g", "", "");
    opts.optflag("h", "", "");
    opts.optflag("l", "", "");
    opts.optflag("p", "", "");
    opts.optflag("q", "", "");
    opts.optflag("r", "", "");
    opts.optflag("s", "", "");
    opts.optflag("u", "", "");
    opts.optflag("w", "", "");
    opts.optflag("x", "", "");

    opts.optopt("n", "newer", "lines of vertical list", "LINES");
    opts.optopt("o", "older", "add prompt to left of input field", "PROMPT");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => { m }
        Err(f) => { panic!(f.to_string()) }
    };

    let newer = match matches.opt_str("n") {
        Some(newer_path) => {
            match std::fs::metadata(Path::new(&*newer_path)) {
                Ok(metadata) => metadata.mtime(),
                _ => 0
            }
        },
        _ => 0
    };

    let older = match matches.opt_str("o") {
        Some(older_path) => {
            match std::fs::metadata(Path::new(&*older_path)) {
                Ok(metadata) => metadata.mtime(),
                _ => 0
            }
        },
        _ => 0
    };

    Config {
        a: matches.opt_present("a"),
        b: matches.opt_present("b"),
        c: matches.opt_present("c"),
        d: matches.opt_present("d"),
        e: matches.opt_present("e"),
        f: matches.opt_present("f"),
        g: matches.opt_present("g"),
        h: matches.opt_present("h"),
        l: matches.opt_present("l"),
        n: matches.opt_present("n"),
        o: matches.opt_present("o"),
        p: matches.opt_present("p"),
        q: matches.opt_present("q"),
        r: matches.opt_present("r"),
        s: matches.opt_present("s"),
        u: matches.opt_present("u"),
        w: matches.opt_present("w"),
        x: matches.opt_present("x"),
        newer: newer,
        older: older,
        paths: matches.free,
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let config = get_config(args);

    let mut any_match = false;

    if config.paths.len() == 0 {
        let stdin = std::io::stdin();
        let input_items: Vec<String> = stdin.lock().lines().map(|x| x.unwrap()).collect();
        for item in &input_items {
            let path = Path::new(&*item);
            if check(path, &config) {
                println!("{}", path.file_name().unwrap().to_str().unwrap());
                any_match = true;
            }
        }
    } else {
        for filename in config.paths.first().unwrap_or(&"".to_string()).split(":") {
            let path = Path::new(&*filename);
            match std::fs::metadata(path) {
                Ok(metadata) => {
                    if config.l && metadata.is_dir() {
                        let dir_entries: Vec<String> = match read_dir(path) {
                            Ok(entries) => entries.map(|x| x.unwrap().path().to_str().unwrap().to_string()).collect(),
                            Err(_) => vec![]
                        };
                        for entry in dir_entries {
                            let entry_path = Path::new(&entry);
                            if check(entry_path, &config) {
                                println!("{}", entry_path.file_name().unwrap().to_str().unwrap());
                                any_match = true;
                            }
                        }
                    } else {
                        if check(path, &config) {
                            println!("{}", path.file_name().unwrap().to_str().unwrap());
                            any_match = true;
                        }
                    }
                },
                _ => continue
            }
        }
    }

    if any_match {
        exit(0);
    } else {
        exit(1);
    }
}

trait MetadataPerms {
    fn is_readable(&self) -> bool;
    fn is_writeable(&self) -> bool;
    fn is_executable(&self) -> bool;
    fn has_sgid(&self) -> bool;
    fn has_suid(&self) -> bool;
}

impl MetadataPerms for Metadata {
    fn is_readable(&self) -> bool {
        self.mode() & R_OK as u32 != 0
    }
    fn is_writeable(&self) -> bool {
        self.mode() & W_OK as u32 != 0
    }
    fn is_executable(&self) -> bool {
        self.mode() & X_OK as u32 != 0
    }
    fn has_sgid(&self) -> bool {
        self.mode() & S_ISGID as u32 != 0
    }
    fn has_suid(&self) -> bool {
        self.mode() & S_ISUID as u32 != 0
    }
}

fn check(path: &Path, config: &Config) -> bool {
    let checks = (
        |config: &Config, path: &Path| -> bool { config.a || !path.file_name().unwrap_or(OsStr::new("")).to_str().unwrap_or("").starts_with(".") },
        |config: &Config, path: &Path| -> bool { !config.e || path.exists() },
        |config: &Config, metadata: &Metadata| -> bool { !config.b || metadata.file_type().is_block_device() },
        |config: &Config, metadata: &Metadata| -> bool { !config.c || metadata.file_type().is_char_device() },
        |config: &Config, metadata: &Metadata| -> bool { !config.f || metadata.file_type().is_file() },
        |config: &Config, metadata: &Metadata| -> bool { !config.h || metadata.file_type().is_symlink() },
        |config: &Config, metadata: &Metadata| -> bool { !config.p || metadata.file_type().is_fifo() },
        |config: &Config, metadata: &Metadata| -> bool { !config.d || metadata.file_type().is_dir() },
        |config: &Config, metadata: &Metadata| -> bool { !config.g || metadata.has_sgid() },
        |config: &Config, metadata: &Metadata| -> bool { !config.n || metadata.mtime() > config.newer },
        |config: &Config, metadata: &Metadata| -> bool { !config.o || metadata.mtime() < config.older },
        |config: &Config, metadata: &Metadata| -> bool { !config.r || metadata.is_readable() },
        |config: &Config, metadata: &Metadata| -> bool { !config.s || metadata.size() > 0 },
        |config: &Config, metadata: &Metadata| -> bool { !config.u || metadata.has_suid() },
        |config: &Config, metadata: &Metadata| -> bool { !config.w || metadata.is_writeable() },
        |config: &Config, metadata: &Metadata| -> bool { !config.x || metadata.is_executable() },
    );

    let mut result = true;
    if !checks.0(&config, &path) { result = false; }
    if !checks.1(&config, &path) { result = false; }

    match std::fs::metadata(path) {
        Ok(metadata) => {
            if !checks.2(&config, &metadata) { result = false; }
            if !checks.3(&config, &metadata) { result = false; }
            if !checks.4(&config, &metadata) { result = false; }
            if !checks.5(&config, &metadata) { result = false; }
            if !checks.6(&config, &metadata) { result = false; }
            if !checks.7(&config, &metadata) { result = false; }
            if !checks.8(&config, &metadata) { result = false; }
            if !checks.9(&config, &metadata) { result = false; }
            if !checks.10(&config, &metadata) { result = false; }
            if !checks.11(&config, &metadata) { result = false; }
            if !checks.12(&config, &metadata) { result = false; }
            if !checks.13(&config, &metadata) { result = false; }
            if !checks.14(&config, &metadata) { result = false; }
            if !checks.15(&config, &metadata) { result = false; }
            if result && config.q { exit(0); }
            result
        },
        _ => result
    }
}

#[cfg(test)]
mod tests {
    use super::check;
    use super::get_config;
    use std::path::Path;

    #[test]
    fn test_check_no_filter() {
        assert!(check(Path::new("/dev/null"), &get_config(vec!["rutest".to_string()])));
    }

    #[test]
    fn test_check_dot_files() {
        assert!(!check(Path::new("/home/jespino/.bashrc"), &get_config(vec!["rutest".to_string()])));
        assert!(check(Path::new("/home/jespino/.bashrc"), &get_config(vec!["rutest".to_string(), "-a".to_string()])));
    }

    #[test]
    fn test_check_existing_files() {
        assert!(check(Path::new("not-existing-file"), &get_config(vec!["rutest".to_string()])));
        assert!(!check(Path::new("not-existing-file"), &get_config(vec!["rutest".to_string(), "-e".to_string()])));
    }

    #[test]
    fn test_check_block_devices() {
        assert!(check(Path::new("/dev/sda"), &get_config(vec!["rutest".to_string()])));
        assert!(!check(Path::new("/dev/sda"), &get_config(vec!["rutest".to_string(), "-b".to_string()])));
    }

    #[test]
    fn test_check_char_devices() {
        assert!(check(Path::new("/dev/random"), &get_config(vec!["rutest".to_string()])));
        assert!(!check(Path::new("/dev/random"), &get_config(vec!["rutest".to_string(), "-c".to_string()])));
    }

    #[test]
    fn test_check_regular_file() {
        assert!(check(Path::new("/etc/fstab"), &get_config(vec!["rutest".to_string()])));
        assert!(!check(Path::new("/etc/fstab"), &get_config(vec!["rutest".to_string(), "-f".to_string()])));
    }

    #[test]
    fn test_check_symbolic_link() {
        assert!(check(Path::new("/dev/stdin"), &get_config(vec!["rutest".to_string()])));
        assert!(!check(Path::new("/dev/stdin"), &get_config(vec!["rutest".to_string(), "-h".to_string()])));
    }

    #[test]
    fn test_check_fifo() {
        assert!(check(Path::new("/var/run/dmeventd-server"), &get_config(vec!["rutest".to_string()])));
        assert!(!check(Path::new("/var/run/dmeventd-server"), &get_config(vec!["rutest".to_string(), "-p".to_string()])));
    }

    #[test]
    fn test_check_socket() {
        // TODO
        // assert!(check(Path::new("/var/run/systemd/notify"), &get_config(vec!["rutest".to_string()])));
        // assert!(!check(Path::new("/var/run/systemd/notify"), &get_config(vec!["rutest".to_string(), "-h".to_string()])));
        assert!(false);
    }

    #[test]
    fn test_check_directory() {
        assert!(check(Path::new("/dev"), &get_config(vec!["rutest".to_string()])));
        assert!(!check(Path::new("/dev"), &get_config(vec!["rutest".to_string(), "-d".to_string()])));
    }

    #[test]
    fn test_check_suid() {
        // TODO
        // |config: &Config, metadata: &Metadata| -> bool { !config.u || metadata.has_suid() },
        assert!(false);
    }
    #[test]
    fn test_check_sgid() {
        // TODO
        // |config: &Config, metadata: &Metadata| -> bool { !config.g || metadata.has_sgid() },
        assert!(false);
    }

    #[test]
    fn test_check_newer() {
        // TODO
        // |config: &Config, metadata: &Metadata| -> bool { !config.n || metadata.mtime() > config.newer },
        assert!(false);
    }

    #[test]
    fn test_check_older() {
        // TODO
        // |config: &Config, metadata: &Metadata| -> bool { !config.o || metadata.mtime() < config.older },
        assert!(false);
    }

    #[test]
    fn test_check_readable() {
        // TODO
        // |config: &Config, metadata: &Metadata| -> bool { !config.r || metadata.is_readable() },
        assert!(false);
    }

    #[test]
    fn test_check_writeable() {
        // TODO
        // |config: &Config, metadata: &Metadata| -> bool { !config.w || metadata.is_writeable() },
        assert!(false);
    }

    #[test]
    fn test_check_executable() {
        // TODO
        // |config: &Config, metadata: &Metadata| -> bool { !config.x || metadata.is_executable() },
        assert!(false);
    }

    #[test]
    fn test_check_not_empty() {
        // TODO
        // |config: &Config, metadata: &Metadata| -> bool { !config.s || metadata.size() > 0 },
        assert!(false);
    }
}
