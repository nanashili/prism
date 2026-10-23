use eframe::egui::{self, Color32, FontId, TextFormat};

pub(super) fn highlight(
    text: &str,
    font: FontId,
    dark: bool,
    scheme: &str,
) -> egui::text::LayoutJob {
    let [base, keyword, number, string, comment] = colors(dark, scheme);
    let mut job = egui::text::LayoutJob::default();
    let mut offset = 0;
    while offset < text.len() {
        let tail = &text[offset..];
        let (length, color) = if tail.starts_with("//") || tail.starts_with(';') {
            (tail.len(), comment)
        } else if tail.starts_with('"') || tail.starts_with('\'') {
            let quote = tail.as_bytes()[0];
            let mut escaped = false;
            let mut end = tail.len();
            for (index, byte) in tail.bytes().enumerate().skip(1) {
                if byte == quote && !escaped {
                    end = index + 1;
                    break;
                }
                escaped = byte == b'\\' && !escaped;
            }
            (end, string)
        } else {
            let first = tail.chars().next().unwrap_or(' ');
            if first.is_alphanumeric() || first == '_' {
                let end = tail
                    .char_indices()
                    .find(|(_, character)| !character.is_alphanumeric() && *character != '_')
                    .map(|(index, _)| index)
                    .unwrap_or(tail.len());
                let word = &tail[..end];
                let color = if first.is_ascii_digit() {
                    number
                } else if KEYWORDS.contains(&word) {
                    keyword
                } else {
                    base
                };
                (end, color)
            } else {
                (first.len_utf8(), base)
            }
        };
        job.append(
            &text[offset..offset + length],
            0.0,
            TextFormat {
                font_id: font.clone(),
                color,
                ..Default::default()
            },
        );
        offset += length;
    }
    job
}

const KEYWORDS: &[&str] = &[
    "if", "else", "for", "while", "return", "switch", "case", "break", "continue", "void", "int",
    "char", "long", "float", "double", "struct", "class", "const", "static", "unsigned", "fn",
    "let", "mut", "pub", "impl", "use", "match", "enum", "unsafe", "async", "await", "true",
    "false", "null", "nullptr", "NULL", "self", "Self", "mov", "ldr", "str", "ret", "bl",
];

fn colors(dark: bool, scheme: &str) -> [Color32; 5] {
    let base = if dark {
        Color32::from_gray(200)
    } else {
        Color32::from_gray(58)
    };
    if scheme == "Classic" {
        return [
            base,
            Color32::from_rgb(86, 156, 214),
            Color32::from_rgb(181, 206, 168),
            Color32::from_rgb(206, 145, 120),
            Color32::from_rgb(106, 153, 85),
        ];
    }
    if scheme == "Quiet" {
        let text = if dark {
            Color32::from_gray(237)
        } else {
            Color32::from_gray(23)
        };
        let muted = if dark {
            Color32::from_gray(168)
        } else {
            Color32::from_gray(82)
        };
        let faint = if dark {
            Color32::from_gray(107)
        } else {
            Color32::from_gray(143)
        };
        return [base, text, muted, muted, faint];
    }
    if dark {
        [
            base,
            Color32::from_rgb(198, 120, 221),
            Color32::from_rgb(209, 154, 102),
            Color32::from_rgb(152, 195, 121),
            Color32::from_rgb(92, 99, 112),
        ]
    } else {
        [
            base,
            Color32::from_rgb(166, 38, 164),
            Color32::from_rgb(152, 104, 1),
            Color32::from_rgb(80, 161, 79),
            Color32::from_rgb(160, 161, 167),
        ]
    }
}
