pub(crate) mod bench_utils;

use criterion::{Criterion, criterion_group, criterion_main};

use crate::bench_utils::{
    bench_utils_methods,
    bench_utils_models::{DataSet, Heuristic},
};

fn run_all_heuristics_ghjava(c: &mut Criterion) {
    let variants = [
        Heuristic::LazySimple,
        Heuristic::LazyGreedy200,
        Heuristic::LazyGreedy1000,
        Heuristic::Simple,
        Heuristic::Greedy200,
    ];

    let dataset_projects: Vec<DataSet> = DataSet::GhJava(None)
        .get_all_projects_of_dataset()
        .into_iter()
        .map(|name| DataSet::GhJava(Some(name)))
        .collect();

    for dataset in dataset_projects {
        bench_utils_methods::run_all_heuristics_for_dataset(c, dataset, &variants);
    }
}

fn run_all_heuristics_defects4j(c: &mut Criterion) {
    let variants = [
        Heuristic::LazySimple,
        Heuristic::LazyGreedy200,
        Heuristic::LazyGreedy1000,
        Heuristic::Simple,
        Heuristic::Greedy200,
    ];

    let dataset_projects: Vec<DataSet> = DataSet::Defects4J(None)
        .get_all_projects_of_dataset()
        .into_iter()
        .map(|name| DataSet::Defects4J(Some(name)))
        .collect();

    for dataset in dataset_projects {
        bench_utils_methods::run_all_heuristics_for_dataset(c, dataset, &variants);
    }
}

// Make sure the event_paranoid is set for this session, 0 or 1 should suffice.
// sudo sysctl -w kernel.perf_event_paranoid=0
criterion_group!(
    name = defects4j_all;
    config = Criterion::default()
        .sample_size(10)
        .configure_from_args();
    targets = run_all_heuristics_defects4j
);
criterion_group!(
    name = ghjava_all;
    config = Criterion::default()
        .sample_size(10)
        .configure_from_args();
    targets = run_all_heuristics_ghjava
);
criterion_main!(defects4j_all, ghjava_all);
