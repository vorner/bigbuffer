extern crate humansize;
extern crate structopt;
#[macro_use]
extern crate structopt_derive;

use std::io;
use std::io::prelude::*;
use std::sync::atomic::{AtomicUsize, ATOMIC_USIZE_INIT, Ordering};
use std::sync::mpsc;
use std::thread::{self, Builder as Thread};
use std::time::Duration;

use humansize::{FileSize, file_size_opts as size};
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
struct Options {
    #[structopt(short="v", long="verbose", help="Provides some progress output")]
    verbose: bool,
    #[structopt(short="n", long="name", help="Names the buffer in the progress output")]
    name: Option<String>,
    #[structopt(short="u", long="update", help="Progress update interval, in seconds",
                default_value="5", parse(try_from_str))]
    update: u64,
    #[structopt(help="Size in MiB")]
    size: usize,
}

static READ_CNT: AtomicUsize = ATOMIC_USIZE_INIT;
static WRITE_CNT: AtomicUsize = ATOMIC_USIZE_INIT;

fn fs(val: u64) -> String {
    match val.file_size(size::CONVENTIONAL) {
        Ok(s) => s,
        Err(s) => s,
    }
}

fn main() {
    let options = Options::from_args();

    let (sender, receiver) = mpsc::sync_channel::<Vec<u8>>(options.size);
    let len = 1024*1024;
    let reader = Thread::new()
        .name("Reader".to_owned())
        .spawn(move || {
            loop {
                let mut buffer = Vec::new();
                io::stdin()
                    .take(len)
                    .read_to_end(&mut buffer)
                    .unwrap();
                if (buffer.len() as u64) < len {
                    break;
                }
                READ_CNT.fetch_add(1, Ordering::Relaxed);
                sender.send(buffer)
                    .unwrap();
            }
        })
        .unwrap();

    let writer = Thread::new()
        .name("Writer".to_owned())
        .spawn(move || {
            for buffer in receiver {
                io::stdout()
                    .write_all(&buffer)
                    .unwrap();
                WRITE_CNT.fetch_add(1, Ordering::Relaxed);
            }
        })
        .unwrap();

    if options.verbose {
        Thread::new()
            .name("Progress".to_owned())
            .spawn(move || {
                let name = options.name
                    .map(|n| format!("{}: ", n))
                    .unwrap_or_else(String::new);
                let mut last_read = 0;
                let mut last_written = 0;
                loop {
                    thread::sleep(Duration::from_secs(options.update as u64));
                    let read_raw = READ_CNT.load(Ordering::Relaxed) as u64;
                    let read = read_raw * len;
                    let diff_read = (read - last_read) / options.update;
                    let written_raw = WRITE_CNT.load(Ordering::Relaxed) as u64;
                    let written = written_raw * len;
                    let diff_written = (written - last_written) / options.update;
                    let fill = read_raw - written_raw;
                    eprintln!("{}Read {} ({}/s), written {} ({}/s), fill {}%",
                              name, fs(read), fs(diff_read), fs(written), fs(diff_written),
                              (100 * fill) / (options.size as u64));
                    last_read = read;
                    last_written = written;
                }
            })
            .unwrap();
    }

    reader.join()
        .unwrap();
    writer.join()
        .unwrap();
}
