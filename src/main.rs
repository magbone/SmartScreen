mod macos;
mod sensor;

use std::env;
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;

use sensor::{Reading, Source};

fn print_usage() {
    println!(
        "smart-screen\n\
         \n\
         Options:\n\
           --demo                 Show simulated temperature and humidity\n\
           --serial <path>        Read JSON or 'environment: 24 57' lines\n\
           --url <http-url>       Poll a Pico W HTTP JSON endpoint\n\
           --help                 Show this help\n\
         \n\
         Without a source option, common Pico/JDY serial ports are detected."
    );
}

fn parse_source() -> Source {
    let mut args = env::args().skip(1);
    let Some(argument) = args.next() else {
        return Source::Auto;
    };

    match argument.as_str() {
        "--demo" => Source::Demo,
        "--serial" => {
            let path = args.next().unwrap_or_else(|| {
                eprintln!("--serial requires a device path");
                std::process::exit(2);
            });
            Source::Serial(path)
        }
        "--url" => {
            let url = args.next().unwrap_or_else(|| {
                eprintln!("--url requires an HTTP URL");
                std::process::exit(2);
            });
            Source::Http(url)
        }
        "--help" | "-h" => {
            print_usage();
            std::process::exit(0);
        }
        _ => {
            eprintln!("Unknown option: {argument}");
            print_usage();
            std::process::exit(2);
        }
    }
}

fn main() {
    let source = parse_source();
    let reading = Arc::new(RwLock::new(Reading::waiting()));
    let worker_reading = Arc::clone(&reading);

    thread::spawn(move || sensor::run(source, worker_reading));

    let ui_reading = Arc::clone(&reading);
    thread::spawn(move || loop {
        macos::refresh(&ui_reading);
        thread::sleep(Duration::from_secs(1));
    });

    macos::run();
}
