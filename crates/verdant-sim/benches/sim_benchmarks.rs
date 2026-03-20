use criterion::{criterion_group, criterion_main, Criterion};
use verdant_core::types::{Timestamp, ZoneId};
use verdant_sim::scenario::{SimConfig, ZoneLayout};
use verdant_sim::sim::Simulation;

fn zone(id: u8) -> ZoneId {
    ZoneId([id, 0, 0, 0])
}

fn make_sim(node_count: usize, zone_count: usize) -> Simulation {
    let side = (node_count as f64).sqrt().ceil() as usize;
    let config = SimConfig {
        node_count,
        zone_count,
        zone_layout: ZoneLayout::Grid {
            rows: side,
            cols: side,
        },
        rf_max_range_m: 500.0,
        training_ticks: 200,
    };
    Simulation::new(config)
}

fn bench_mesh_convergence(c: &mut Criterion) {
    c.bench_function("1000-node mesh convergence", |b| {
        b.iter(|| {
            let mut sim = make_sim(1000, 10);
            sim.deploy_nodes();
            sim.run_until_mesh_converges(15);
        });
    });
}

fn bench_message_throughput(c: &mut Criterion) {
    c.bench_function("50-node 100-tick throughput", |b| {
        b.iter_with_setup(
            || {
                let mut sim = make_sim(50, 5);
                sim.deploy_nodes();
                sim.run_until_mesh_converges(10);
                sim.fast_forward_training(100);
                sim.inject_flood(zone(0), Timestamp::from_secs(500));
                sim.inject_pest(zone(3), Timestamp::from_secs(500));
                sim
            },
            |mut sim| {
                sim.run_for(100);
            },
        );
    });
}

fn bench_vector_graph_update(c: &mut Criterion) {
    c.bench_function("vector graph training (200 ticks, 50 nodes)", |b| {
        b.iter_with_setup(
            || {
                let mut sim = make_sim(50, 5);
                sim.deploy_nodes();
                sim
            },
            |mut sim| {
                sim.fast_forward_training(200);
            },
        );
    });
}

fn bench_crypto_sign_verify(c: &mut Criterion) {
    use verdant_core::traits::PostQuantumCrypto;
    use verdant_qudag::crypto::PqCrypto;

    let crypto = PqCrypto::generate();
    let data = b"Verdant mesh governance vote payload";
    let sig = crypto.sign(data).unwrap();
    let pk = crypto.signing_public_key();

    c.bench_function("dilithium sign", |b| {
        b.iter(|| {
            crypto.sign(data).unwrap();
        });
    });

    c.bench_function("dilithium verify", |b| {
        b.iter(|| {
            crypto.verify(data, &sig, &pk).unwrap();
        });
    });
}

criterion_group!(
    benches,
    bench_mesh_convergence,
    bench_message_throughput,
    bench_vector_graph_update,
    bench_crypto_sign_verify,
);
criterion_main!(benches);
