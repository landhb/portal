extern crate portal_lib as portal;
use criterion::{criterion_group, criterion_main, Criterion};
use mockstream::MockStream;
use portal::{protocol::Protocol, Direction, Portal};
use portal::{NO_PROGRESS_CALLBACK, NO_VERIFY_CALLBACK};
use std::fs::File;
use std::io::{Read, Write};
use std::time::{Duration, Instant};
use tempdir::TempDir;

#[derive(Clone)]
pub struct MockTcpStream {
    pub inner: MockStream,
}

impl Read for MockTcpStream {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, std::io::Error> {
        self.inner.read(buf)
    }
}

impl Write for MockTcpStream {
    fn write(&mut self, buf: &[u8]) -> Result<usize, std::io::Error> {
        self.inner.push_bytes_to_read(buf);
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
    let mut receiver = Portal::init(dir, "id".to_string(), pass).unwrap();

    // sender
    let dir = Direction::Sender;
    let pass = "test".to_string();
    let mut sender = Portal::init(dir, "id".to_string(), pass).unwrap();

    // Get a key
    let state = sender.state.take().unwrap();
    let key = Protocol::derive_key(state, &receiver.exchange).unwrap();

    // Give each side the key
    sender.set_key(key.clone());
    receiver.set_key(key);

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

    // encrypt & send the file
    let total_size = sender
        .send_file(stream, &file_path_str, NO_PROGRESS_CALLBACK)
        .unwrap();
    assert_eq!(total_size, size as usize);
}

fn bench_file_receiver(c: &mut Criterion) {
    // Fake TCP stream
    let mut stream = MockTcpStream {
        inner: MockStream::new(),
    };

    // Init receiver
    let (mut sender, mut receiver) = setup();

    // Create test directory
    let tmp_dir = TempDir::new("sending").unwrap();
    let out_dir = TempDir::new("receiving").unwrap();

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
                let metatada = receiver
                    .recv_file(
                        &mut stream,
                        out_dir.path(),
                        NO_VERIFY_CALLBACK,
                        NO_PROGRESS_CALLBACK,
                    )
                    .unwrap();

                // End timing
                total_time += start.elapsed();
                assert_eq!(metatada.filesize, 100_000);
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
                let metatada = receiver
                    .recv_file(
                        &mut stream,
                        out_dir.path(),
                        NO_VERIFY_CALLBACK,
                        NO_PROGRESS_CALLBACK,
                    )
                    .unwrap();

                // End timing
                total_time += start.elapsed();
                assert_eq!(metatada.filesize, 1_000_000);
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
                let metatada = receiver
                    .recv_file(
                        &mut stream,
                        out_dir.path(),
                        NO_VERIFY_CALLBACK,
                        NO_PROGRESS_CALLBACK,
                    )
                    .unwrap();

                // End timing
                total_time += start.elapsed();
                assert_eq!(metatada.filesize, 100_000_000);
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
                let metatada = receiver
                    .recv_file(
                        &mut stream,
                        out_dir.path(),
                        NO_VERIFY_CALLBACK,
                        NO_PROGRESS_CALLBACK,
                    )
                    .unwrap();

                // End timing
                total_time += start.elapsed();
                assert_eq!(metatada.filesize, 500_000_000);
            }
            total_time
        })
    });

    group.finish();
}

criterion_group!(benches, bench_file_receiver);
criterion_main!(benches);
