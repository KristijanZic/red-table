use std::{
    collections::HashSet,
    hint::black_box,
    io::{self, Write},
    os::unix::ffi::OsStrExt,
    path::PathBuf,
    time::Instant,
};

const ENTRIES: usize = 100_000;
const SAMPLES: usize = 9;

fn main() {
    let paths = (0..ENTRIES)
        .map(|index| PathBuf::from(format!("/photos/session/image-{index:06}.jpg")))
        .collect::<Vec<_>>();
    let marked = (0..ENTRIES).step_by(3).collect::<HashSet<_>>();
    println!("selection pipeline: {ENTRIES} entries, median of {SAMPLES}");
    measure("selected-only rebuild", || {
        let visible = (0..ENTRIES)
            .filter(|index| marked.contains(index))
            .collect::<Vec<_>>();
        black_box(visible.len());
    });
    measure("NUL result serialization", || {
        let mut output = Vec::with_capacity(4 * 1024 * 1024);
        serialize_paths(&mut output, black_box(&paths)).unwrap();
        black_box(output.len());
    });
}

fn serialize_paths(writer: &mut impl Write, paths: &[PathBuf]) -> io::Result<()> {
    for path in paths {
        writer.write_all(path.as_os_str().as_bytes())?;
        writer.write_all(&[0])?;
    }
    writer.flush()
}

fn measure(label: &str, operation: impl Fn()) {
    operation();
    let mut samples = Vec::with_capacity(SAMPLES);
    for _ in 0..SAMPLES {
        let started = Instant::now();
        operation();
        samples.push(started.elapsed());
    }
    samples.sort_unstable();
    println!("{label}: {:.2?}", samples[SAMPLES / 2]);
}
