use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};
use fluorite_gba::gba::Gba;
use fluorite_gba::VideoInterface;
use std::cell::RefCell;
use std::rc::Rc;

struct BenchmarkHardware {}
impl VideoInterface for BenchmarkHardware {
    fn render(&mut self, _: &[u8]) {}
}

fn create_gba() -> Gba<BenchmarkHardware> {
    let bios = include_bytes!("../roms/gba_bios.bin");
    let rom = include_bytes!("../roms/yoshi_dma.gba");

    let dummy = Rc::new(RefCell::new(BenchmarkHardware {}));

    let mut gba = Gba::new(dummy, bios, rom);
    gba.skip_bios();
    // skip initialization of the ROM to get to a stabilized scene
    for _ in 0..60 {
        gba.frame();
    }
    gba
}

pub fn performance_benchmark(c: &mut Criterion) {
    c.bench_function("run_60_frames", |b| {
        b.iter_batched(
            // setup
            || create_gba(),
            // bencher
            |mut gba| {
                for _ in 0..60 {
                    black_box(gba.frame())
                }
            },
            BatchSize::SmallInput,
        )
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(10);
    targets = performance_benchmark
}
criterion_main!(benches);
