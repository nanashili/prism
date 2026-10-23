use super::model::{Command, Control, Group, Row};
use crate::settings::{Settings, permissions::Tool};

pub(super) fn groups(s: &mut Settings) -> Vec<Group<'_>> {
    let row = |tool, title, description| {
        let mut row = Row::new(title, description);
        row.control = Control::Permission(tool, s.permissions.level(tool));
        row
    };
    let groups = vec![
        Group::new(
            "Reading",
            vec![
                row(
                    Tool::Read,
                    "Read the open file",
                    "Functions, strings, code and bytes in the app you have open.",
                ),
                row(
                    Tool::View,
                    "Move around the window",
                    "Open functions, switch views and jump to addresses for you.",
                ),
            ],
        ),
        Group::new(
            "Changing your project",
            vec![
                row(
                    Tool::Rename,
                    "Rename functions and variables",
                    "Saved in the project. You can undo any rename.",
                ),
                row(
                    Tool::Notes,
                    "Add notes, comments and bookmarks",
                    "Saved in the project next to the code.",
                ),
                row(
                    Tool::Patch,
                    "Patch bytes",
                    "Writes a patched copy. The original file is never touched.",
                ),
            ],
        ),
        Group::new(
            "Running things",
            vec![
                row(
                    Tool::Gpu,
                    "Run the app to record a GPU trace",
                    "Starts the program under Instruments for a few seconds.",
                ),
                row(
                    Tool::Debug,
                    "Run the app in the debugger",
                    "Starts the program and stops at breakpoints.",
                ),
                row(
                    Tool::Shell,
                    "Run commands on your computer",
                    "The agent’s own terminal tools, outside Prism.",
                ),
            ],
        ),
    ];
    let mut groups = groups;
    groups.push(Group::new(
        "Your answers",
        vec![
            Row::new(
                "Remember “Always allow” for each project",
                "When you pick Always allow in a chat, it sticks for that project only.",
            )
            .toggle(&mut s.desktop.remember_answers),
            Row::new(
                "Forget remembered answers",
                "Ask again next time, in every project.",
            )
            .button("Forget all", Command::ForgetAnswers),
        ],
    ));
    groups
}
