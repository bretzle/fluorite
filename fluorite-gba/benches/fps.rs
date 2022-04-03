use criterion::{criterion_group, criterion_main, Criterion};
use fluorite_common::flume::unbounded;
use fluorite_gba::{gba::Gba, AudioInterface};

struct DummyAudio;

impl AudioInterface for DummyAudio {
    fn write(&mut self, _: [i16; 2]) {}
}

fn run_gba(gba: &mut Gba) {
    gba.reset();
    for _ in 0..32 {
        gba.run(0x44940);
    }
}

fn criterion_benchmark(c: &mut Criterion) {
    let (_, rx) = unbounded();
    let mut gba = Gba::new(rx);
    let mut dummy = DummyAudio;
    Gba::load_audio(&mut dummy);
    gba.load_rom("C:\\Users\\johnf\\CLionProjects\\fluorite\\roms\\pokemon\\Pokemon Emerald.gba");
    gba.reset();

    c.bench_function("fps pokemon", |b| b.iter(|| run_gba(&mut gba)));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
