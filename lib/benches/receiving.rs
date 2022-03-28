extern crate portal_lib as portal;
use criterion::{criterion_group, criterion_main, Criterion};
use portal::{Direction, Portal};
use std::fs::File;
use std::io::{Read, Write};
use std::time::{Duration, Instant};
use tempdir::TempDir;

#[derive(Clone, Debug)]
pub struct MockTcpStream {
    pub data: Vec<u8>,
}

impl Read for MockTcpStream {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, std::io::Error> {
        let size: usize = std::cmp::min(self.data.len(), buf.len());
        if size == 0 {
            return Ok(size);
        }
        buf[..size].copy_from_slice(&self.data[..size]);
        self.data.drain(0..size);
        Ok(size)
    }
}

impl Write for MockTcpStream {
    fn write(&mut self, buf: &[u8]) -> Result<usize, std::io::Error> {
        self.data.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<(), std::io::Error> {
        Ok(())
    }
}

/// Common to all sender tests
fn setup() -> (Portal, Portal) {
    // receiver
    let dir = Direction::Receiver;
    let pass = "test".to_string();
    let (mut receiver, receiver_msg) = Portal::init(dir, "id".to_string(), pass, None);

    // sender
    let dir = Direction::Sender;
    let pass = "test".to_string();
    let (mut sender, sender_msg) = Portal::init(dir, "id".to_string(), pass, None);

    // we need a key to be able to encrypt
    receiver.derive_key(sender_msg.as_slice()).unwrap();
    sender.derive_key(receiver_msg.as_slice()).unwrap();

    (sender, receiver)
}

/// Create a file of size, tempdir must live longer
/// since it is removed once it is dropped
fn send_file(sender: &mut Portal, stream: &mut MockTcpStream, dir: &TempDir, size: u64) {
    let file_path = dir.path().join("testfile.raw");
    let file_path_str = file_path.to_str().unwrap().to_owned();
    let mut tmp_file = File::create(file_path).unwrap();
    writeln!(tmp_file, "Arbitrary text here.").unwrap();

    // Set the file size
    tmp_file.set_len(size).unwrap();

    // encrypt the file
    let mut file = sender.load_file(&file_path_str).unwrap();
    file.encrypt().unwrap();

    // communicate the necessary state info
    // for the peer to be able to decrypt the file
    file.sync_file_state(stream).unwrap();

    // send the file over the stream
    for data in file.get_chunks(portal::CHUNK_SIZE) {
        stream.write(&data).unwrap();
    }
}

fn bench_file_receiver(c: &mut Criterion) {
    // Fake TCP stream
    let mut stream = MockTcpStream {
        data: Vec::with_capacity(100_000),
    };

    // Init receiver
    let (mut sender, receiver) = setup();

    // Create test directory
    let tmp_dir = TempDir::new("sending").unwrap();
    let results_path = tmp_dir.path().join("results.raw");

    // Benchmark creating the file and downloading the data
    // + the decryption. 100k
    send_file(&mut sender, &mut stream, &tmp_dir, 100_000);
    let backup = stream.clone();
    c.bench_function("receive & decrypt 100k", |b| {
        b.iter_custom(|iters| {
            let mut total_time = Duration::ZERO;
            for _i in 0..iters {
                // Each iteration must have a new stream to consume
                stream = backup.clone();

                // Begin timing after the setup is done
                let start = Instant::now();

                // use download_file to read in the file data
                let mut new_file = receiver
                    .create_file(results_path.to_str().unwrap(), 100_000)
                    .unwrap();

                new_file.download_file(&mut stream, |_x| {}).unwrap();

                new_file.decrypt().unwrap(); // should not panic

                // End timing
                total_time += start.elapsed();
            }
            total_time
        })
    });

    // 1M
    send_file(&mut sender, &mut stream, &tmp_dir, 1_000_000);
    let backup = stream.clone();
    c.bench_function("receive & decrypt 1M", |b| {
        b.iter_custom(|iters| {
            let mut total_time = Duration::ZERO;
            for _i in 0..iters {
                // Each iteration must have a new stream to consume
                stream = backup.clone();

                // Begin timing after the setup is done
                let start = Instant::now();

                // use download_file to read in the file data
                let mut new_file = receiver
                    .create_file(results_path.to_str().unwrap(), 1_000_000)
                    .unwrap();

                new_file.download_file(&mut stream, |_x| {}).unwrap();

                new_file.decrypt().unwrap(); // should not panic

                // End timing
                total_time += start.elapsed();
            }
            total_time
        })
    });

    // Configure Criterion.rs with larger measurement times
    // for larger files.
    let mut group = c.benchmark_group("larger-files");
    group.measurement_time(core::time::Duration::new(200, 0));
    group.sample_size(10);

    // 100M
    send_file(&mut sender, &mut stream, &tmp_dir, 100_000_000);
    let backup = stream.clone();
    group.bench_function("receive & decrypt 100M", |b| {
        b.iter_custom(|iters| {
            let mut total_time = Duration::ZERO;
            for _i in 0..iters {
                // Each iteration must have a new stream to consume
                stream = backup.clone();

                // Begin timing after the setup is done
                let start = Instant::now();

                // use download_file to read in the file data
                let mut new_file = receiver
                    .create_file(results_path.to_str().unwrap(), 100_000_000)
                    .unwrap();

                new_file.download_file(&mut stream, |_x| {}).unwrap();

                new_file.decrypt().unwrap(); // should not panic

                // End timing
                total_time += start.elapsed();
            }
            total_time
        })
    });

    // 500M
    send_file(&mut sender, &mut stream, &tmp_dir, 500_000_000);
    let backup = stream.clone();
    group.bench_function("receive & decrypt 500M", |b| {
        b.iter_custom(|iters| {
            let mut total_time = Duration::ZERO;
            for _i in 0..iters {
                // Each iteration must have a new stream to consume
                stream = backup.clone();

                // Begin timing after the setup is done
                let start = Instant::now();

                // use download_file to read in the file data
                let mut new_file = receiver
                    .create_file(results_path.to_str().unwrap(), 500_000_000)
                    .unwrap();

                new_file.download_file(&mut stream, |_x| {}).unwrap();

                new_file.decrypt().unwrap(); // should not panic

                // End timing
                total_time += start.elapsed();
            }
            total_time
        })
    });

    group.finish();
}

criterion_group!(benches, bench_file_receiver);
criterion_main!(benches);
