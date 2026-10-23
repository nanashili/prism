use super::model::{Command, Group, Lead, Row};
use crate::settings::Settings;
use crate::ui::agent_catalog::Catalog;
use eframe::egui;

pub(super) fn groups<'a>(
    s: &'a mut Settings,
    catalog: &Catalog,
    browsing: bool,
    custom_tool_server: &'a mut Option<String>,
) -> Vec<Group<'a>> {
    let selected = crate::ui::agent_picker::id(s).to_owned();
    let mut here = Vec::new();
    let mut more = Vec::new();
    for entry in &catalog.entries {
        let installed = catalog.installed.contains(&entry.id);
        let description = if installed {
            format!(
                "Version {}. Speaks the Agent Client Protocol.",
                entry.version
            )
        } else {
            "Not on this Mac. Prism downloads it when you start a chat.".into()
        };
        let mut row = Row::new(&entry.name, description);
        row.lead = Some(Lead::Agent(entry.id.clone()));
        if selected == entry.id {
            row.tag = Some("Default");
        } else {
            row = row.button(
                "Make default",
                Command::Agent(entry.id.clone(), entry.name.clone()),
            );
        }
        if installed {
            let hidden = s.conversation.hidden_agents.contains(&entry.id);
            here.push(row.status("Installed", true).button(
                if hidden {
                    "Show in composer"
                } else {
                    "Hide from composer"
                },
                Command::AgentVisibility(entry.id.clone()),
            ));
        } else if browsing || more.len() < 2 {
            more.push(row);
        }
    }
    if !s.agent.trim().is_empty() {
        let logo = crate::ui::agent_picker::custom_id(s);
        if !catalog.installed.contains(logo) {
            let hidden = s.conversation.hidden_agents.iter().any(|id| id == logo);
            here.push(
                Row::new("Your agent", "Your configured ACP command.").button(
                    if hidden {
                        "Show in composer"
                    } else {
                        "Hide from composer"
                    },
                    Command::AgentVisibility(logo.into()),
                ),
            );
        }
    }
    here.push(
        Row::new(
            "Look for agents again",
            "After installing one from the terminal.",
        )
        .button("Rescan", Command::Rescan),
    );
    more.push(
        Row::new(
            format!("Browse all {} agents", catalog.entries.len()),
            "From the open agent registry, the same one Zed uses.",
        )
        .button(
            if browsing { "Show fewer" } else { "Browse…" },
            Command::BrowseAgents,
        ),
    );
    let c = &mut s.conversation;
    let tool_server = custom_tool_server.get_or_insert_with(|| c.custom_tool_server.clone());
    more.push(
        Row::new(
            "Add your own agent",
            "Any command that speaks the Agent Client Protocol.",
        )
        .input(&mut c.custom_agent, "my-agent --acp")
        .button("Add", Command::CustomAgent),
    );
    vec![
        Group::new("On this Mac", here).foot("The composer shows installed agents and your custom agent, grouped by provider. Hide agents here to keep them out of the picker. Sign in through each agent’s own CLI."),
        Group::new("More agents", more),
        Group::new("Tools agents get", vec![
            Row::new("Prism", "Open functions, switch views, rename and take notes in this window.")
                .toggle(&mut c.prism_tools).image(crate::ui::branding::LOGO),
            Row::new("Prism analysis", "Uses the selected analysis engine and its saved results.")
                .toggle(&mut c.rea_tools).image(crate::ui::branding::LOGO),
            Row::new("IDA Pro", "Uses your IDA license through its MCP plugin.")
                .toggle(&mut c.ida_tools).image(egui::include_image!("../../../resources/upstream/engines/ida.png"))
                .blocked("A separate IDA MCP server is not available yet. IDA analysis is available through REA."),
            Row::new("Add a tool server", "Any stdio MCP server. Every agent gets its tools. Clear it and choose Add to remove it.")
                .input(tool_server, "npx my-mcp-server").button("Add", Command::CustomToolServer),
        ]).foot("Tool servers use the Model Context Protocol. Every agent gets the ones that are on."),
        Group::new("Ask about code", vec![
            Row::new("Model", "The selected agent’s quick answers currently use the chat’s model.")
                .select(&mut c.ask_model, &["Same as the chat"])
                .blocked("A separate model for quick answers is not available yet."),
            Row::new("Fast mode", "Quicker answers from agents that offer it, like Claude. It can cost more.")
                .toggle(&mut c.ask_fast).blocked("Use the agent’s own model and effort controls in the composer."),
        ]).foot("Select lines in any code view, then Ask or Explain. These only change those quick answers, not the chat."),
        Group::new("Running agents", vec![
            Row::new("Keep agents running between chats", "Replies start faster. Uses a little more memory.")
                .toggle(&mut c.keep_agents_running),
            Row::new("Start the agent when a project opens", "So it’s ready before you ask.")
                .toggle(&mut c.start_agent_with_project).blocked("Agents currently start on demand to keep memory use low."),
        ]),
    ]
}
