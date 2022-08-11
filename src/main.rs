use std::fs::File;
use std::io::prelude::*;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::{sync_channel, Receiver},
};
use std::time::Duration;
use std::{thread, time};

use anyhow::{Context, Result};

const MAIN_SLEEP_TIME: Duration = Duration::from_micros(2500);

static DONE: AtomicBool = AtomicBool::new(false);

#[derive(Debug)]
struct DataContainer {
    internal_count: u64,
    fake_vector: [u8; 10240],
}

fn main() -> Result<()> {
    ctrlc::set_handler(|| DONE.store(true, Ordering::SeqCst))?;

    let mut loop_counter: u64 = 0;
    let mut fake_counter: u8 = 0;
    let (datasender, datareceiver) = sync_channel::<DataContainer>(4);

    let write_thread = thread::spawn(move || write_thread(datareceiver));

    while !DONE.load(Ordering::Relaxed) {
        let loop_start = time::Instant::now();

        let data = DataContainer {
            internal_count: loop_counter,
            fake_vector: [fake_counter; 10240],
        };
        loop_counter += 1;
        fake_counter = fake_counter.wrapping_add(1);

        if datasender.send(data).is_err() {
            // The receiving side hung up!
            // Bounce out of hte loop to see what error it had.
            break;
        }

        let loop_end = time::Instant::now();

        let dt = loop_end - loop_start;

        if dt < MAIN_SLEEP_TIME {
            thread::sleep(MAIN_SLEEP_TIME - dt);
        }
    }

    drop(datasender);
    write_thread.join().expect("Couldn't join writer")?;

    Ok(())
}

fn write_thread(receiver: Receiver<DataContainer>) -> Result<()> {
    let mut file = File::create("output.bin").context("Couldn't create output file")?;

    let mut start = time::Instant::now();

    while let Ok(received_data) = receiver.recv() {
        let wait_done = time::Instant::now();
        let wait = wait_done - start;

        file.write_all(&received_data.fake_vector)
            .context("Couldn't write output")?;
        let wrote = time::Instant::now() - wait_done;

        println!(
            "Wrote {}! (waited {} us, wrote {} us)",
            received_data.internal_count,
            wait.as_micros(),
            wrote.as_micros()
        );

        // Account for the time spent blocking for input
        start = time::Instant::now();
    }
    Ok(())
}
