extern crate portal_lib as portal;
use criterion::{criterion_group, criterion_main, Criterion};
use portal::{Direction, Portal};
use std::fs::File;
use std::io::Write;
use tempdir::TempDir;

/// Common to all sender tests
fn setup() -> Portal {
    // receiver
    let dir = Direction::Receiver;
    let pass = "test".to_string();
    let (_receiver, receiver_msg) = Portal::init(dir, "id".to_string(), pass, None);

    // sender
    let dir = Direction::Sender;
    let pass = "test".to_string();
    let (mut sender, _sender_msg) = Portal::init(dir, "id".to_string(), pass, None);

    // Confirm
    sender.derive_key(receiver_msg.as_slice()).unwrap();
    sender
}

/// Create a file of size, tempdir must live longer
/// since it is removed once it is dropped
fn create_file(dir: &TempDir, size: u64) -> String {
    let file_path = dir.path().join("testfile.raw");
    let file_path_str = file_path.as_path().to_str().unwrap().to_owned();
    let mut tmp_file = File::create(file_path).unwrap();
    writeln!(tmp_file, "Arbitrary text here.").unwrap();

    // Set the file size
    tmp_file.set_len(size).unwrap();

    file_path_str
}

fn bench_file_sender(c: &mut Criterion) {
    // Init sender
    let sender = setup();

    // Create test directory
    let tmp_dir = TempDir::new("sending").unwrap();

    // Benchmark loading the file and iterating over the chunks
    // this allows to compare chunking vs single pass encryption
    let path = create_file(&tmp_dir, 100_000);
    c.bench_function("encrypt & send 100k", |b| {
        b.iter(|| {
            let mut file = sender.load_file(&path).unwrap();
            file.encrypt().unwrap();
            let mut total_size = 0;
            for v in file.get_chunks(portal::CHUNK_SIZE) {
                assert!(v.len() <= portal::CHUNK_SIZE);
                total_size += v.len();
            }
            assert!(total_size >= 100_000);
        })
    });

    // 1M
    let path = create_file(&tmp_dir, 1_000_000);
    c.bench_function("encrypt & send 1M", |b| {
        b.iter(|| {
            let mut file = sender.load_file(&path).unwrap();
            file.encrypt().unwrap();
            let mut total_size = 0;
            for v in file.get_chunks(portal::CHUNK_SIZE) {
                assert!(v.len() <= portal::CHUNK_SIZE);
                total_size += v.len();
            }
            assert!(total_size >= 1_000_000);
        })
    });

    // 100M
    let path = create_file(&tmp_dir, 100_000_000);
    c.bench_function("encrypt & send 100M", |b| {
        b.iter(|| {
            let mut file = sender.load_file(&path).unwrap();
            file.encrypt().unwrap();
            let mut total_size = 0;
            for v in file.get_chunks(portal::CHUNK_SIZE) {
                assert!(v.len() <= portal::CHUNK_SIZE);
                total_size += v.len();
            }
            assert!(total_size >= 100_000_000);
        })
    });

    // Configure Criterion.rs with larger measurement times
    /* for larger files.
    let mut group = c.benchmark_group("larger-files");
    group.measurement_time(core::time::Duration::new(60,0));

    //500M
    let path = create_file(&tmp_dir, 500_000_000);
    group.bench_function("encrypt & send 500M", |b| b.iter(|| {
        let mut file = sender.load_file(&path).unwrap();
        file.encrypt().unwrap();
        let mut total_size = 0;
        for v in file.get_chunks(portal::CHUNK_SIZE) {
            assert!(v.len() <= portal::CHUNK_SIZE);
            total_size += v.len();
        }
        assert!(total_size >= 500_000_000);
    }));


    group.finish(); */
}

criterion_group!(benches, bench_file_sender);
criterion_main!(benches);
