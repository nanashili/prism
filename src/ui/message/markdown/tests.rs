use super::*;

#[test]
fn removes_only_fenced_language_hints() {
    let cases = [
        ("hello café `rust`.\n", "hello café `rust`.\n"),
        ("```\nlet x = 2;\n```\n", "```\nlet x = 2;\n```\n"),
        ("```rust\nlet café = 2;\n```\n", "```\nlet café = 2;\n```\n"),
        (
            "~~~cpp option\r\nint x;\r\n~~~\r\n",
            "~~~\nint x;\r\n~~~\r\n",
        ),
        (
            "   ```rust\n   let x = 2;\n   ```\n",
            "   ```\n   let x = 2;\n   ```\n",
        ),
        (
            "    ```rust\n    let x = 2;\n    ```\n",
            "    ```rust\n    let x = 2;\n    ```\n",
        ),
        ("```rust\nreturn", "```\nreturn"),
        ("```rust", "```"),
        (
            "> ```rust\n> let café = 2;\n> ```\n",
            "> ```\n> let café = 2;\n> ```\n",
        ),
        (
            "🦀\n```rust\nx\n```\n\nΔ\n~~~c++\ny\n~~~\n",
            "🦀\n```\nx\n```\n\nΔ\n~~~\ny\n~~~\n",
        ),
    ];
    for (source, expected) in cases {
        assert_eq!(without_syntax_hints(source), expected, "{source:?}");
    }
}

#[test]
fn keeps_unmodified_markdown_borrowed() {
    for source in ["", "plain", "```\nx\n```\n", "    ```rust\n"] {
        assert!(matches!(without_syntax_hints(source), Cow::Borrowed(_)));
    }
}

#[test]
#[ignore = "manual release benchmark; run with --release --ignored --nocapture"]
fn benchmark_without_syntax_hints() {
    use std::hint::black_box;
    use std::time::Instant;

    for (blocks, iterations) in [(8, 2000), (4096, 10)] {
        let source = (0..blocks)
            .map(|index| {
                format!(
                    "Function {index}:\n\n```cpp\n{}return result;\n```\n\n",
                    "    result += object->entries[index].value;\n".repeat(4)
                )
            })
            .collect::<String>();
        let expected = source.replace("```cpp\n", "```\n");
        assert_eq!(without_syntax_hints(&source), expected);
        let mut samples = Vec::new();
        for _ in 0..7 {
            let start = Instant::now();
            for _ in 0..iterations {
                black_box(without_syntax_hints(black_box(&source)));
            }
            samples.push(start.elapsed().as_nanos() as f64 / iterations as f64);
        }
        samples.sort_by(f64::total_cmp);
        println!(
            "without_syntax_hints: blocks={blocks} source_bytes={} iterations={iterations} samples=7 median_ns={:.0} min_ns={:.0} max_ns={:.0}",
            source.len(),
            samples[3],
            samples[0],
            samples[6]
        );
    }
}
