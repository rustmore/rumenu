extern crate libc;
extern crate x11;
extern crate getopts;

mod matches;
mod ui;

use ui::UI;
use matches::simple_match;
use matches::fuzzy_match;
use matches::dmenu_match;
use std::str::FromStr;
use std::io::BufRead;
use std::io::BufReader;
use std::fs::File;
use std::env;
use std::iter::Iterator;

use getopts::Options;

const VERSION: &'static str = "0.0.1";

pub struct Settings {
    topbar: bool,
    fast: bool,
    casesensitive: bool,
    lines: u32,
    prompt: String,
    font: String,
    normbgcolor: String,
    normfgcolor: String,
    selbgcolor: String,
    selfgcolor: String,
    cache_file: String,
    matcher: String,
}

impl Settings {
    fn new() -> Settings{
        Settings {
            topbar: false,
            fast: false,
            casesensitive: true,
            lines: 0,
            prompt: String::new(),
            font: "fixed".to_string(),
            normbgcolor: "rgb:22/22/22".to_string(),
            normfgcolor: "rgb:bb/bb/bb".to_string(),
            selbgcolor: "rgb:00/55/77".to_string(),
            selfgcolor: "rgb:ee/ee/ee".to_string(),
            cache_file: "-".to_string(),
            matcher: "simple".to_string(),
        }
    }
}

struct Status {
    text: String,
    matches: Vec<String>,
    items: Vec<String>,
    selected: String,
    page: u32,
    settings: Settings,
}

fn readitems(settings: &Settings) -> Vec<String> {
    let mut items = vec![];
    let mut input_items: Vec<_>;

    if settings.cache_file == "-" {
        let stdin = std::io::stdin();
        input_items = stdin.lock().lines().collect();
    } else {
        input_items = match File::open(settings.cache_file.clone()) {
            Ok(file) => BufReader::new(file).lines().collect(),
            Err(e) => panic!("{}", e)
        }
    }
    for item in input_items {
        items.push(item.unwrap())
    }
    items
}

fn main () {
    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();

    let mut settings = Settings::new();

    let mut opts = Options::new();
    opts.optflag("v", "version", "show version");
    opts.optflag("b", "topbar", "show topbar");
    opts.optflag("f", "fast", "fast start");
    opts.optflag("h", "help", "show help");
    opts.optflag("i", "caseinsensitive", "activate case insensitive");

    opts.optopt("l", "lines", "lines of vertical list", "LINES");
    opts.optopt("c", "cache", "cache file with available commands", "CACHE_FILE");
    opts.optopt("p", "prompt", "add prompt to left of input field", "PROMPT");
    opts.optopt("m", "matcher", "select matcher function", "simple|dmenu|fuzzy");
    opts.optopt("", "font", "font or font set", "FONT");
    opts.optopt("", "background", "normal background color", "NBG");
    opts.optopt("", "foreground", "normal foreground color", "NFG");
    opts.optopt("", "sbackground", "selected background color", "SBG");
    opts.optopt("", "sforeground", "selected foreground color", "SFG");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => { m }
        Err(f) => { panic!(f.to_string()) }
    };

    if matches.opt_present("v") {
        println!("rumenu-{}, © 2015 Jesús Espino, see LICENSE for details", VERSION);
        return;
    }

    if matches.opt_present("h") {
        let brief = format!("Usage: {} [options]", program);
        print!("{}", opts.usage(&brief));
        return;
    }

    settings.topbar = matches.opt_present("b");
    settings.fast = matches.opt_present("f");
    settings.casesensitive = !matches.opt_present("i");

    match matches.opt_str("l") {
        Some(lines_str) => {
            match u32::from_str(lines_str.trim()) {
                Ok(l) => { settings.lines = l }
                Err(f) => { println!("{}", f.to_string()); return }
            }
        }
        None => {
            settings.lines = 0
        }
    }

    settings.prompt =  matches.opt_str("p").unwrap_or(String::new());
    settings.matcher =  matches.opt_str("m").unwrap_or("simple".to_string());
    settings.font = matches.opt_str("font").unwrap_or("fixed".to_string());
    settings.normbgcolor = matches.opt_str("background").unwrap_or("rgb:22/22/22".to_string());
    settings.normfgcolor = matches.opt_str("foreground").unwrap_or("rgb:bb/bb/bb".to_string());
    settings.selbgcolor = matches.opt_str("sbackground").unwrap_or("rgb:00/55/77".to_string());
    settings.selfgcolor = matches.opt_str("sforeground").unwrap_or("rgb:ee/ee/ee".to_string());
    settings.cache_file = matches.opt_str("cache").unwrap_or("-".to_string());

    let mut ui = UI::new(&settings);

    let items = readitems(&settings);

    let mut status = Status {
        text: "".to_string(),
        matches: vec![],
        items: items,
        selected: "".to_string(),
        page: 0,
        settings: settings,
    };

    status.items.sort();

    if status.settings.matcher == "fuzzy" {
        status.matches = fuzzy_match(&status.text, &status.items);
    } else if status.settings.matcher == "dmenu" {
        status.matches = dmenu_match(&status.text, &status.items);
    } else {
        status.matches = simple_match(&status.text, &status.items);
    }

    status.selected = status.matches.first().unwrap_or(&"".to_string()).clone();
    ui.run(status);
}
