extern crate ansi_term;
extern crate errno;
extern crate libc;
extern crate linefeed;
extern crate nix;
extern crate regex;
extern crate shlex;
extern crate sqlite;
extern crate time;
extern crate yaml_rust;

#[macro_use]
extern crate nom;

use std::env;
use std::rc::Rc;

// use std::thread;
// use std::time::Duration;

use ansi_term::Colour::{Red, Green};

use linefeed::Command;
use linefeed::{Reader, ReadResult};

use std::io::{self, Read};

mod builtins;
mod completers;
mod execute;
mod jobs;
mod parsers;
mod tools;
mod binds;
mod rcfile;
mod history;


fn main() {
    if env::args().len() > 1 {
        println!("does not support args yet.");
        return;
    }

    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    let user = env::var("USER").unwrap();
    let home = tools::get_user_home();
    let env_path = env::var("PATH").unwrap();
    let dir_bin_cargo = format!("{}/.cargo/bin", home);
    let env_path_new = ["/usr/local/bin".to_string(),
                        env_path,
                        dir_bin_cargo,
                        "/Library/Frameworks/Python.framework/Versions/3.6/bin".to_string(),
                        "/Library/Frameworks/Python.framework/Versions/3.5/bin".to_string(),
                        "/Library/Frameworks/Python.framework/Versions/3.4/bin".to_string(),
                        "/Library/Frameworks/Python.framework/Versions/2.7/bin".to_string()]
            .join(":");
    env::set_var("PATH", &env_path_new);
    rcfile::load_rcfile();

    let isatty: bool = unsafe { libc::isatty(0) == 1 };
    if !isatty {
        let mut buffer = String::new();
        let stdin = io::stdin();
        let mut handle = stdin.lock();
        handle.read_to_string(&mut buffer).expect("read to str error");
        execute::run_procs(buffer);
        return;
    }

    let mut rl;
    match Reader::new("cicada") {
        Ok(x) => rl = x,
        Err(e) => {
            // non-tty will raise errors here
            println!("Reader Error: {:?}", e);
            return;
        }
    }
    history::init(&mut rl);
    rl.set_completer(Rc::new(completers::CCDCompleter));

    rl.define_function("up-key-function", Rc::new(binds::UpKeyFunction));
    rl.bind_sequence(binds::SEQ_UP_KEY, Command::from_str("up-key-function"));

    let mut previous_dir = String::new();
    let mut painter;
    let mut status = 0;
    loop {
        if status == 0 {
            painter = Green;
        } else {
            painter = Red;
        }

        let _current_dir = env::current_dir().unwrap();
        let current_dir = _current_dir.to_str().unwrap();
        let _tokens: Vec<&str> = current_dir.split("/").collect();

        let last = _tokens.last().unwrap();
        let pwd: String;
        if last.to_string() == "" {
            pwd = String::from("/");
        } else if current_dir == home {
            pwd = String::from("~");
        } else {
            pwd = last.to_string();
        }

        let prompt = tools::wrap_seq_chars(
            format!("{}@{}: {}$ ",
                    painter.paint(user.to_string()),
                    painter.paint("cicada"),
                    painter.paint(pwd)));
        rl.set_prompt(prompt.as_str());

        match rl.read_line() {
            Ok(ReadResult::Input(line)) => {
                let mut cmd;
                if line.trim() == "exit" {
                    break;
                } else if line.trim() == "" {
                    continue;
                } else if line.trim() == "version" {
                    println!("Cicada v{} by @mitnk", VERSION);
                    continue;
                } else if line.trim() == "bash" {
                    cmd = String::from("bash --rcfile ~/.bash_profile");
                } else {
                    cmd = line.clone();
                }

                let tsb_spec = time::get_time();
                let tsb = (tsb_spec.sec as f64) + tsb_spec.nsec as f64 / 1000000000.0;

                tools::pre_handle_cmd_line(&mut cmd);
                // special cases needing extra context
                if line.trim().starts_with("cd") {
                    status = builtins::cd::run(cmd, &mut previous_dir);
                } else {
                    // normal cases
                    status = execute::run_procs(cmd);
                }
                let tse_spec = time::get_time();
                let tse = (tse_spec.sec as f64) + tse_spec.nsec as f64 / 1000000000.0;
                history::add(&mut rl, line.as_str(), status, tsb, tse);
            }
            Ok(ReadResult::Eof) => {
                println!("");
                continue;
            }
            Ok(ReadResult::Signal(s)) => {
                println!("readline signal: {:?}", s);
            }
            Err(e) => {
                println!("readline error: {:?}", e);
            }
        }
    }
}
