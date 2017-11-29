extern crate structopt;
#[macro_use]
extern crate structopt_derive;

use std::io;
use std::io::prelude::*;
use std::sync::atomic::{AtomicUsize, ATOMIC_USIZE_INIT, Ordering};
use std::sync::mpsc;
use std::thread::{self, Builder as Thread};
use std::time::Duration;

use structopt::StructOpt;

#[derive(StructOpt)]
struct Options {
    #[structopt(short="v", long="verbose", help="Provide some progress output")]
    verbose: bool,
    #[structopt(help="Size in MiB")]
    size: usize,
}

static READ_CNT: AtomicUsize = ATOMIC_USIZE_INIT;
static WRITE_CNT: AtomicUsize = ATOMIC_USIZE_INIT;

fn main() {
    let options = Options::from_args();

    let (sender, receiver) = mpsc::sync_channel::<Vec<u8>>(options.size);
    let reader = Thread::new()
        .name("Reader".to_owned())
        .spawn(move || {
            loop {
                let mut buffer = Vec::new();
                io::stdin()
                    .take(1024*1024)
                    .read_to_end(&mut buffer)
                    .unwrap();
                if buffer.len() == 0 {
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
                let mut last_read = 0;
                let mut last_written = 0;
                loop {
                    thread::sleep(Duration::from_secs(1));
                    let read = READ_CNT.load(Ordering::Relaxed);
                    let diff_read = read - last_read;
                    let written = WRITE_CNT.load(Ordering::Relaxed);
                    let diff_written = written - last_written;
                    let fill = read - written;
                    eprintln!("Read {} MB ({}/s), written {} MB ({}/s), fill {}%",
                              read, diff_read, written, diff_written, (100 * fill) / options.size);
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
