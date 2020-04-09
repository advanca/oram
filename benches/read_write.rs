use criterion::{
    black_box, criterion_group, criterion_main, AxisScale, BenchmarkId, Criterion,
    PlotConfiguration,
};
use oram::SqrtOram;
use rand::Rng;

static BLOCK_SIZE: usize = 16;

fn initialization(c: &mut Criterion) {
    let plot_config = PlotConfiguration::default().summary_scale(AxisScale::Logarithmic);

    let mut group = c.benchmark_group("initialization");
    group.plot_config(plot_config);

    for n in [16, 32, 64, 128, 256, 512].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(n), n, |b, &n| {
            b.iter(|| {
                black_box(SqrtOram::new(n, BLOCK_SIZE));
            });
        });
    }

    group.finish();
}

fn put_same_location(c: &mut Criterion) {
    let plot_config = PlotConfiguration::default().summary_scale(AxisScale::Logarithmic);
    let mut group = c.benchmark_group("put same location");
    group.plot_config(plot_config);

    for n in [16, 32, 64, 128, 256, 512, 1024, 2048, 4096].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(n), n, |b, &n| {
            let mut oram = SqrtOram::new(n, BLOCK_SIZE);
            b.iter(|| {
                oram.put(0, vec![0; BLOCK_SIZE]);
            });
        });
    }
}

fn get_same_location(c: &mut Criterion) {
    let plot_config = PlotConfiguration::default().summary_scale(AxisScale::Logarithmic);
    let mut group = c.benchmark_group("get same location");
    group.plot_config(plot_config);

    for n in [16, 32, 64, 128, 256, 512, 1024, 2048, 4096].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(n), n, |b, &n| {
            let mut oram = SqrtOram::new(n, BLOCK_SIZE);
            oram.put(0, vec![0; BLOCK_SIZE]);
            b.iter(|| {
                black_box(oram.get(0));
            });
        });
    }
}

fn put_random_location(c: &mut Criterion) {
    let plot_config = PlotConfiguration::default().summary_scale(AxisScale::Logarithmic);
    let mut group = c.benchmark_group("put random location");
    group.plot_config(plot_config);

    for n in [16, 32, 64, 128, 256, 512, 1024, 2048, 4096].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(n), n, |b, &n| {
            let mut oram = SqrtOram::new(n, BLOCK_SIZE);
            let k = rand::thread_rng().gen_range(0, n) as u32;
            b.iter(|| {
                oram.put(k, vec![0; BLOCK_SIZE]);
            });
        });
    }
}

fn get_random_location(c: &mut Criterion) {
    let plot_config = PlotConfiguration::default().summary_scale(AxisScale::Logarithmic);
    let mut group = c.benchmark_group("get random location");
    group.plot_config(plot_config);

    for n in [16, 32, 64, 128, 256, 512, 1024, 2048, 4096].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(n), n, |b, &n| {
            let mut oram = SqrtOram::new(n, BLOCK_SIZE);
            let k = rand::thread_rng().gen_range(0, n) as u32;
            oram.put(k, vec![0; BLOCK_SIZE]);
            b.iter(|| {
                black_box(oram.get(k));
            });
        });
    }
}

criterion_group!(
    benches,
    initialization,
    put_same_location,
    get_same_location,
    put_random_location,
    get_random_location
);
criterion_main!(benches);
