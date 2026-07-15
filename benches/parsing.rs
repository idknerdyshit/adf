mod support;

use criterion::{BatchSize, BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use quick_xml::Reader;
use quick_xml::events::Event;
use std::hint::black_box;
use std::io::Cursor;
use support::{Workload, workloads};

fn borrowed_parse(criterion: &mut Criterion) {
    let inputs = workloads();
    let mut group = criterion.benchmark_group("borrowed_parse");

    for workload in &inputs {
        group.throughput(Throughput::Bytes(workload.xml.len() as u64));
        group.bench_with_input(workload.name, workload, |bencher, workload| {
            bencher.iter(|| black_box(adf::parse(black_box(workload.xml.as_str())).unwrap()));
        });
    }

    group.finish();
}

fn input_ownership(criterion: &mut Criterion) {
    let inputs = workloads();
    let mut group = criterion.benchmark_group("input_ownership");

    for workload in selected(&inputs, &["typical", "batch_100"]) {
        group.throughput(Throughput::Bytes(workload.xml.len() as u64));

        group.bench_with_input(
            BenchmarkId::new("borrowed", workload.name),
            workload,
            |bencher, workload| {
                bencher.iter(|| black_box(adf::parse(black_box(workload.xml.as_str())).unwrap()));
            },
        );
        group.bench_with_input(
            BenchmarkId::new("owned", workload.name),
            workload,
            |bencher, workload| {
                bencher.iter_batched(
                    || workload.xml.clone(),
                    |input| black_box(adf::parse_owned(black_box(input)).unwrap()),
                    BatchSize::LargeInput,
                );
            },
        );
        group.bench_with_input(
            BenchmarkId::new("bytes", workload.name),
            workload,
            |bencher, workload| {
                bencher.iter(|| {
                    black_box(adf::parse_bytes(black_box(workload.xml.as_bytes())).unwrap())
                });
            },
        );
        group.bench_with_input(
            BenchmarkId::new("reader", workload.name),
            workload,
            |bencher, workload| {
                bencher.iter(|| {
                    let reader = Cursor::new(black_box(workload.xml.as_bytes()));
                    black_box(adf::parse_reader(reader).unwrap())
                });
            },
        );
    }

    group.finish();
}

fn raw_tree(criterion: &mut Criterion) {
    let inputs = workloads();
    let mut group = criterion.benchmark_group("raw_tree");

    for workload in selected(&inputs, &["typical", "batch_100"]) {
        group.throughput(Throughput::Bytes(workload.xml.len() as u64));

        group.bench_with_input(
            BenchmarkId::new("lazy_root_only", workload.name),
            workload,
            |bencher, workload| {
                bencher.iter_batched(
                    || adf::parse(workload.xml.as_str()).unwrap(),
                    |document| {
                        black_box(document.root());
                        black_box(document);
                    },
                    BatchSize::LargeInput,
                );
            },
        );
        group.bench_with_input(
            BenchmarkId::new("parse_and_root", workload.name),
            workload,
            |bencher, workload| {
                bencher.iter(|| {
                    let document = adf::parse(black_box(workload.xml.as_str())).unwrap();
                    black_box(document.root());
                    black_box(document)
                });
            },
        );
    }

    group.finish();
}

fn tokenizer_floor(criterion: &mut Criterion) {
    let inputs = workloads();
    let mut group = criterion.benchmark_group("tokenizer_floor");

    for workload in selected(
        &inputs,
        &["typical", "extension_mixed", "batch_100", "batch_1000"],
    ) {
        group.throughput(Throughput::Bytes(workload.xml.len() as u64));
        group.bench_with_input(workload.name, workload, |bencher, workload| {
            bencher.iter(|| black_box(scan_quick_xml(black_box(workload.xml.as_str()))));
        });
    }

    group.finish();
}

fn scan_quick_xml(input: &str) -> usize {
    let mut reader = Reader::from_str(input);
    {
        let config = reader.config_mut();
        config.trim_text(false);
        config.check_comments = true;
    }

    let mut events = 0_usize;
    loop {
        let event = reader.read_event().unwrap();
        events += 1;
        if matches!(event, Event::Eof) {
            return events;
        }
    }
}

fn selected<'a>(inputs: &'a [Workload], names: &[&str]) -> Vec<&'a Workload> {
    names
        .iter()
        .map(|name| {
            inputs
                .iter()
                .find(|workload| workload.name == *name)
                .expect("benchmark workload should exist")
        })
        .collect()
}

criterion_group!(
    parsing,
    borrowed_parse,
    input_ownership,
    raw_tree,
    tokenizer_floor
);
criterion_main!(parsing);
