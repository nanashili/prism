use super::{Graph, Instructions};
use serde_json::json;
use std::hint::black_box;
use std::time::{Duration, Instant};

fn legacy_lines(rows: &[(u64, &str)], ordered: bool, lower: u64, upper: u64) -> Vec<String> {
    if ordered {
        let start = rows.partition_point(|(address, _)| *address < lower);
        rows[start..]
            .iter()
            .take_while(|(address, _)| *address < upper)
            .take(12)
            .map(|(_, line)| (*line).to_owned())
            .collect()
    } else {
        rows.iter()
            .filter(|(address, _)| *address >= lower && *address < upper)
            .take(12)
            .map(|(_, line)| (*line).to_owned())
            .collect()
    }
}

#[test]
fn block_lines_preserve_provider_order_limits_and_address_boundaries() {
    let text: Vec<_> = (0..80)
        .map(|index| format!("instruction {index}"))
        .collect();
    let rows: Vec<_> = text
        .iter()
        .enumerate()
        // Include duplicate addresses and scrambled input, and exceed the cap.
        .map(|(index, line)| ((index * 17 % 37) as u64, line.as_str()))
        .collect();
    let mut instructions = Instructions::new(rows.clone(), 128);
    for lower in 0..40 {
        for upper in 0..40 {
            assert_eq!(
                instructions.block_lines(lower, upper),
                legacy_lines(&rows, false, lower, upper),
                "range {lower}..{upper}"
            );
        }
    }
}

#[test]
fn sorted_and_empty_assembly_keep_the_direct_search_path() {
    for rows in [
        vec![],
        vec![(0, "zero"), (4, "four"), (4, "duplicate"), (8, "eight")],
    ] {
        let mut instructions = Instructions::new(rows.clone(), 128);
        assert!(instructions.by_address.is_none());
        for (lower, upper) in [(0, 9), (4, 8), (8, 8), (8, 4), (9, u64::MAX)] {
            assert_eq!(
                instructions.block_lines(lower, upper),
                legacy_lines(&rows, true, lower, upper)
            );
        }
    }
}

#[test]
fn dossier_graph_keeps_unsorted_instruction_text_and_references() {
    let graph = Graph::from_dossier(
        &json!({
            "basic_blocks": [
                {"start":"0x10", "end":"0x20", "successors":["0x20"]},
                {"start":"0x20", "end":"0x24", "successors":[]}
            ],
            "callers":[{"address":"0x100"}],
            "callees":[{"address":"0x200"}],
            "callers_scanned":false
        }),
        "0x20: outside\nignored\n0x18: second address first\n0x10: first address second\n0x18: duplicate\n",
    )
    .unwrap();
    assert_eq!(
        graph.blocks[0].lines,
        ["second address first", "first address second", "duplicate"]
    );
    assert_eq!(graph.blocks[0].successors, ["0x20"]);
    assert_eq!(graph.blocks[1].lines, ["outside"]);
    assert_eq!(graph.callers, ["0x100"]);
    assert_eq!(graph.callees, ["0x200"]);
    assert!(graph.callers_deferred);
}

fn graph_lines(
    rows: Vec<(u64, &str)>,
    blocks: usize,
    indexed: bool,
    overlapping: bool,
) -> Vec<Vec<String>> {
    let bounds = |block: usize| {
        if overlapping {
            (0, u64::MAX)
        } else {
            ((block * 32) as u64, ((block + 1) * 32) as u64)
        }
    };
    if indexed {
        let mut instructions = Instructions::new(rows, blocks);
        (0..blocks)
            .map(|block| {
                let (lower, upper) = bounds(block);
                instructions.block_lines(lower, upper)
            })
            .collect()
    } else {
        let ordered = rows.windows(2).all(|pair| pair[0].0 <= pair[1].0);
        (0..blocks)
            .map(|block| {
                let (lower, upper) = bounds(block);
                legacy_lines(&rows, ordered, lower, upper)
            })
            .collect()
    }
}

fn sample(
    rows: &[(u64, &str)],
    blocks: usize,
    indexed: bool,
    overlapping: bool,
    repetitions: usize,
) -> Duration {
    let start = Instant::now();
    for _ in 0..repetitions {
        black_box(graph_lines(
            black_box(rows.to_vec()),
            blocks,
            indexed,
            overlapping,
        ));
    }
    start.elapsed() / repetitions as u32
}

#[test]
fn single_block_and_broad_duplicate_ranges_preserve_early_exit() {
    let mut rows = vec![(4, "duplicate"); 32768];
    rows[0] = (8, "out of order");
    let mut single = Instructions::new(rows.clone(), 1);
    assert!(single.by_address.is_none());
    assert_eq!(single.block_lines(0, 16), legacy_lines(&rows, false, 0, 16));
    let mut multiple = Instructions::new(rows.clone(), 4096);
    assert_eq!(
        multiple.block_lines(0, 16),
        legacy_lines(&rows, false, 0, 16)
    );
    assert_eq!(multiple.block_lines(4, 8), legacy_lines(&rows, false, 4, 8));
    assert!(multiple.by_address.is_none());
}

#[test]
#[ignore = "Release performance benchmark; run alone with --ignored --nocapture"]
fn benchmark_graph_lines() {
    for (case, blocks, row_count, scrambled, overlapping, duplicates) in [
        ("sorted-small", 128, 1024, false, false, false),
        ("scrambled-small", 128, 1024, true, false, false),
        ("sorted-large", 4096, 32768, false, false, false),
        ("scrambled-large", 4096, 32768, true, false, false),
        ("wide-overlapping", 128, 32768, true, true, false),
        ("duplicate-addresses", 128, 32768, true, true, true),
        ("one-wide-block", 1, 32768, true, true, false),
    ] {
        let mut rows: Vec<_> = (0..row_count)
            .map(|index| ((index * 4) as u64, "mov rax, rbx"))
            .collect();
        if duplicates {
            rows.fill((4, "duplicate"));
            rows[0] = (8, "out of order");
        } else if scrambled {
            // Out-of-address-order blocks, as a provider may serialize them.
            rows.reverse();
        }
        assert_eq!(
            graph_lines(rows.clone(), blocks, true, overlapping),
            graph_lines(rows.clone(), blocks, false, overlapping)
        );
        let repetitions = if blocks <= 128 { 50 } else { 2 };
        let mut baseline = Vec::new();
        let mut indexed = Vec::new();
        // Warm both implementations, then alternate measurement order.
        sample(&rows, blocks, false, overlapping, 1);
        sample(&rows, blocks, true, overlapping, 1);
        for trial in 0..9 {
            if trial % 2 == 0 {
                baseline.push(sample(&rows, blocks, false, overlapping, repetitions));
                indexed.push(sample(&rows, blocks, true, overlapping, repetitions));
            } else {
                indexed.push(sample(&rows, blocks, true, overlapping, repetitions));
                baseline.push(sample(&rows, blocks, false, overlapping, repetitions));
            }
        }
        baseline.sort();
        indexed.sort();
        println!(
            "graph_lines case={case} blocks={blocks} rows={} baseline_us={:.3} indexed_us={:.3} speedup={:.2}x",
            rows.len(),
            baseline[4].as_secs_f64() * 1e6,
            indexed[4].as_secs_f64() * 1e6,
            baseline[4].as_secs_f64() / indexed[4].as_secs_f64()
        );
    }
}
