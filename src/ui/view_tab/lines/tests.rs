use super::Lines;
use std::ops::Range;

fn original_ranges(text: &str) -> Vec<Range<usize>> {
    let mut start = 0;
    text.split_inclusive('\n')
        .map(|line| {
            let range = start..start + line.len();
            start += line.len();
            range
        })
        .collect()
}

#[test]
fn indexes_all_lines_without_changing_text_boundaries() {
    for source in ["", "\n", "a", "a\n", "a\n\nb", "café\r\n🦀\nΔ", "\r\n\r\n"] {
        let lines = Lines::new(source);
        let expected = original_ranges(source);
        assert_eq!(lines.len(), expected.len());
        for (index, range) in expected.into_iter().enumerate() {
            assert_eq!(lines.range(index, source.len()), range);
        }
    }
}

#[test]
fn wide_offsets_preserve_ranges_above_u32() {
    if let Some(high) = (u32::MAX as usize).checked_add(1) {
        let lines = Lines::Wide(vec![0, high, high + 2]);
        assert_eq!(lines.len(), 3);
        assert_eq!(lines.range(0, high + 7), 0..high);
        assert_eq!(lines.range(1, high + 7), high..high + 2);
        assert_eq!(lines.range(2, high + 7), high + 2..high + 7);
    }
}

#[test]
#[ignore = "fresh-process memory workload; PRISM_MEMORY_IMPL=baseline|candidate and PRISM_MEMORY_FIXTURE=<text>"]
fn memory_line_index() {
    let implementation =
        std::env::var("PRISM_MEMORY_IMPL").expect("memory line index: valid fixture operation");
    let source = std::fs::read_to_string(
        std::env::var("PRISM_MEMORY_FIXTURE").expect("memory line index: valid fixture operation"),
    )
    .expect("memory line index: valid fixture operation");
    let started = std::time::Instant::now();
    if implementation == "baseline" {
        let lines = original_ranges(&source);
        println!(
            "memory_line_index impl=baseline source_bytes={} lines={} index_bytes={} elapsed_ms={:.3}",
            source.len(),
            lines.len(),
            lines.capacity() * std::mem::size_of::<Range<usize>>(),
            started.elapsed().as_secs_f64() * 1000.0
        );
        std::hint::black_box((&source, &lines));
    } else {
        assert_eq!(implementation, "candidate");
        let lines = Lines::new(&source);
        let bytes = match &lines {
            Lines::Compact(starts) => starts.capacity() * std::mem::size_of::<u32>(),
            Lines::Wide(starts) => starts.capacity() * std::mem::size_of::<usize>(),
        };
        println!(
            "memory_line_index impl=candidate source_bytes={} lines={} index_bytes={bytes} elapsed_ms={:.3}",
            source.len(),
            lines.len(),
            started.elapsed().as_secs_f64() * 1000.0
        );
        std::hint::black_box((&source, &lines));
    }
}
