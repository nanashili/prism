use super::model::{Group, Preview, Row};
use crate::settings::Settings;

pub(super) fn groups(s: &mut Settings) -> Vec<Group<'_>> {
    let preview = s.clone();
    let v = &mut s.views;
    let mut address = Group::new(
        "Addresses",
        vec![
            Row::new("Start with 0x", "").toggle(&mut v.address_prefix),
            Row::new("Capital letters", "For hex digits A to F.").toggle(&mut v.address_upper),
            Row::new(
                "Pad to full width",
                "Every address the same length, so columns line up.",
            )
            .toggle(&mut v.address_pad),
        ],
    );
    address.preview = Some(Preview::Address(preview));
    vec![
        Group::new("Opening a function", vec![
            Row::new("Show first", "What you see when you open a function. Switch any time in the view.")
                .segmented(&mut v.first_view, &["Pseudocode", "Assembly", "Graph", "Hex"]),
            Row::new("Universal apps", "Apps with Apple silicon and Intel code inside.")
                .select(&mut v.universal_apps, &["Ask each time", "Open the one for this Mac"])
                .blocked("Universal apps currently open the architecture of this Mac automatically."),
        ]),
        address,
        Group::new("Hex and lists", vec![
            Row::new("Bytes per row", "In the hex view.").segmented(&mut v.hex_row, &["8", "16", "32"])
                .blocked("The engine currently supplies hex rows. Changing their width is not available yet."),
            Row::new("Strings and symbols to show", "Search always looks through all of them.")
                .select(&mut v.list_limit, &["400", "1,000", "5,000", "All"])
                .blocked("Large lists currently use bounded pages supplied by the analysis engine."),
        ]),
        Group::new("Graph", vec![
            Row::new("Show the minimap", "The small overview in the corner of big graphs.")
                .toggle(&mut v.minimap).blocked("The graph minimap is not available yet."),
            Row::new("Lines per block before folding", "").segmented(&mut v.block_lines, &["4", "8", "16", "All"])
                .blocked("Folding graph blocks is not available yet."),
        ]),
    ]
}

pub(super) fn debugging(s: &mut Settings) -> Vec<Group<'_>> {
    let debugger = super::paths::row("Debugger", &s.lldb, super::model::Path::Debugger);
    let t = &mut s.tools;
    vec![
        Group::new("Debugger", vec![debugger]),
        Group::new(
            "Starting",
            vec![
                Row::new(
                    "Stop at the function you’re looking at",
                    "Puts a breakpoint there before the program starts.",
                )
                .toggle(&mut t.stop_at_function)
                .blocked("Set a function breakpoint in the Debugger view before continuing."),
                Row::new(
                    "Stop at the first instruction",
                    "Pause before any of the program’s code runs.",
                )
                .toggle(&mut t.stop_at_entry)
                .blocked("The current debugger launch does not support stopping at entry."),
                Row::new("Arguments", "Passed to the program each time it starts.")
                    .input(&mut t.program_args, "--verbose"),
            ],
        ),
        Group::new(
            "Showing values",
            vec![
                Row::new(
                    "Variables to show",
                    "Per scope. Big structs fold after this many.",
                )
                .select(&mut t.variable_limit, &["50", "200", "1,000"])
                .blocked("A structured variables view is not available yet."),
            ],
        ),
    ]
}

pub(super) fn gpu(s: &mut Settings) -> Vec<Group<'_>> {
    let t = &mut s.tools;
    let unsupported = "Recording GPU traces is not available in the Rust build yet.";
    vec![
        Group::new(
            "Recording",
            vec![
                Row::new(
                    "Instruments template",
                    "What gets recorded while the app runs.",
                )
                .select(
                    &mut t.gpu_template,
                    &[
                        "Metal System Trace",
                        "Game Performance",
                        "Metal Application",
                    ],
                )
                .blocked(unsupported),
                Row::new("Record for", "Stops sooner if the app quits.")
                    .range(&mut t.gpu_seconds, 5.0, 60.0, 5.0, " s")
                    .blocked(unsupported),
                Row::new(
                    "Frames to skip at the start",
                    "The first frames are often slow while the app loads.",
                )
                .segmented(&mut t.gpu_skip_frames, &["0", "3", "10"])
                .blocked(unsupported),
            ],
        ),
        Group::new(
            "Keeping recordings",
            vec![
                Row::new(
                    "Recordings to keep",
                    "Older ones are deleted. Each can be a few hundred MB.",
                )
                .select(&mut t.keep_recordings, &["1", "3", "10", "All"])
                .blocked(unsupported),
                Row::new(
                    "GPU recordings",
                    super::paths::display(&crate::settings::support_dir().join("recordings")),
                )
                .button("Delete all…", super::model::Command::ClearRecordings)
                .button(
                    "Show in Finder",
                    super::model::Command::Reveal(
                        crate::settings::support_dir().join("recordings"),
                    ),
                ),
            ],
        ),
    ]
}
