use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;

use nbs_rust::{
    audio::{NoteAudio, SampleRate},
    noteblock::Note,
};

fn bench_next_chunk() {
    let frames = vec![[1.0f32, -1.0]; 100000].into_boxed_slice();
    let mut audio = NoteAudio::from_frames(
        frames,
        Note::default(),
        None,
        SampleRate::new(48000).unwrap(),
    );

    while let Some(chunk) = audio.next_chunk() {
        black_box(chunk);
    }
}

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("next_chunk_simd", |b| b.iter(|| bench_next_chunk()));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
