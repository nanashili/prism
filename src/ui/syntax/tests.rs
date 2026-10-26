use super::*;

#[test]
fn preserves_text_and_token_colors() {
    for dark in [PaletteTheme::Light, PaletteTheme::Dark] {
        for scheme in ["Prism", "Classic", "Quiet"] {
            let [base, keyword, number, string, comment] =
                colors(dark == PaletteTheme::Dark, scheme);
            let cases = [
                ("", vec![]),
                ("a + b[index]", vec![("a + b[index]", base)]),
                (
                    "if (café == 42) // comment",
                    vec![
                        ("if", keyword),
                        (" (café == ", base),
                        ("42", number),
                        (") ", base),
                        ("// comment", comment),
                    ],
                ),
                (
                    r#"name = "é\"quoted" + '🦀'"#,
                    vec![
                        ("name = ", base),
                        (r#""é\"quoted""#, string),
                        (" + ", base),
                        ("'🦀'", string),
                    ],
                ),
                (
                    "  mov rax, 0x123 ; trailing",
                    vec![
                        ("  ", base),
                        ("mov", keyword),
                        (" rax, ", base),
                        ("0x123", number),
                        (" ", base),
                        ("; trailing", comment),
                    ],
                ),
                ("\"unterminated\\", vec![("\"unterminated\\", string)]),
            ];
            for (text, expected) in cases {
                let font = FontId::monospace(12.0);
                let job = highlight(text, font.clone(), dark, scheme);
                assert_eq!(job.text, text);
                let expected: Vec<_> = expected
                    .iter()
                    .flat_map(|(token, color)| std::iter::repeat_n(*color, token.len()))
                    .collect();
                let actual: Vec<_> = job
                    .sections
                    .iter()
                    .flat_map(|section| {
                        assert_eq!(section.format.font_id, font);
                        assert!(text.is_char_boundary(section.byte_range.start));
                        assert!(text.is_char_boundary(section.byte_range.end));
                        std::iter::repeat_n(section.format.color, section.byte_range.len())
                    })
                    .collect();
                assert_eq!(actual, expected, "{text:?}, {dark:?}, {scheme}");
            }
        }
    }
}

#[test]
fn coalesces_identical_formatting_without_losing_ranges() {
    let text = "    object->member[index] = other.member + value; return";
    let job = highlight(text, FontId::monospace(12.0), PaletteTheme::Dark, "Prism");
    assert_eq!(job.sections.len(), 2);
    assert_eq!(
        &job.text[job.sections[0].byte_range.clone()],
        &text[..text.find(';').expect(
            "coalesces identical formatting without losing ranges: valid fixture operation"
        )]
    );
    let mut end = 0;
    for section in &job.sections {
        assert_eq!(section.byte_range.start, end);
        end = section.byte_range.end;
    }
    assert_eq!(end, text.len());
}

#[test]
#[ignore = "manual release benchmark; run with --release --ignored --nocapture"]
fn benchmark_syntax_highlighting() {
    // One viewport of typical pseudocode and assembly rows; setup is untimed.
    let lines: Vec<_> = (0..64)
        .map(|index| match index % 4 {
            0 => format!("    result_{index} = object->entries[index].value + offset;"),
            1 => format!("    if (buffer[{index}] == 0x2a) return parse_value(buffer, length);"),
            2 => format!(
                "0x{:08x}    mov rax, [rbp + rdx * 8 - 0x20]",
                0x1000 + index * 16
            ),
            _ => format!(r#"    emit("value_{index}", values[index], flags & mask); // output"#),
        })
        .collect();
    benchmark("viewport", &lines, 10_000);
    // Valid, unusually long generated expressions stress section allocation.
    benchmark(
        "long_expression",
        &["object->entries[index].value + ".repeat(2048)],
        100,
    );
}

fn benchmark(workload: &str, lines: &[String], iterations: usize) {
    use std::hint::black_box;
    use std::time::Instant;

    let font = FontId::monospace(12.0);
    let source_bytes: usize = lines.iter().map(String::len).sum();
    let mut sections = 0;
    let mut section_capacity_bytes = 0;
    for line in lines {
        let job = highlight(line, font.clone(), PaletteTheme::Dark, "Prism");
        section_capacity_bytes +=
            job.sections.capacity() * std::mem::size_of::<egui::text::LayoutSection>();
        sections += job.sections.len();
    }
    let mut samples = Vec::new();
    for _ in 0..7 {
        let start = Instant::now();
        for _ in 0..iterations {
            for line in lines {
                black_box(highlight(
                    black_box(line),
                    font.clone(),
                    PaletteTheme::Dark,
                    "Prism",
                ));
            }
        }
        samples.push(start.elapsed().as_nanos() as f64 / iterations as f64);
    }
    samples.sort_by(f64::total_cmp);
    println!(
        "syntax_highlighting: workload={workload} rows={} source_bytes={source_bytes} iterations={iterations} samples=7 median_ns={:.0} min_ns={:.0} max_ns={:.0} sections={sections} section_capacity_bytes={section_capacity_bytes}",
        lines.len(),
        samples[3],
        samples[0],
        samples[6]
    );
}
