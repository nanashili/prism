use super::app::Prism;
use super::theme::Palette;
use crate::analysis::graph::{Graph, address_value};
use eframe::egui::{self, Align2, FontId, Pos2, Rect, Sense, Stroke, Vec2};
use std::collections::{BTreeMap, BTreeSet};

struct Node {
    block: usize,
    rect: Rect,
}

pub(super) struct GraphView {
    graph: Graph,
    nodes: Vec<Node>,
    edges: Vec<(usize, usize)>,
    size: Vec2,
}

impl GraphView {
    pub fn new(graph: Graph) -> Self {
        let nodes = layout(&graph);
        let size = nodes
            .iter()
            .fold(Vec2::ZERO, |size, node| size.max(node.rect.max.to_vec2()))
            + Vec2::splat(30.0);
        let edges = edge_indices(&graph, &nodes);
        Self {
            graph,
            nodes,
            edges,
            size,
        }
    }
}

impl Prism {
    pub(super) fn refresh_execution_evidence(&mut self, project: &str) -> crate::error::Result<()> {
        if self
            .workspace
            .project
            .as_ref()
            .is_none_or(|opened| opened.target.sha256 != project)
        {
            return Ok(());
        }
        let traces = self.store.execution_traces(project)?;
        for tab in self
            .workspace
            .panes
            .iter_mut()
            .flat_map(|pane| &mut pane.tabs)
        {
            if tab.view == crate::model::View::Graph && !tab.text.is_empty() {
                let mut graph: Graph = serde_json::from_str(&tab.text)?;
                graph.annotate(&traces);
                tab.receive_content(
                    crate::model::View::Graph,
                    "",
                    serde_json::to_string(&graph)?,
                )?;
            }
        }
        Ok(())
    }

    pub(super) fn graph_view(&mut self, ui: &mut egui::Ui) {
        let Some(view) = &self.workspace.graph else {
            self.code(ui);
            return;
        };
        let graph = &view.graph;
        if graph.blocks.is_empty() {
            ui.label("The provider returned no control-flow blocks.");
            return;
        }
        let palette = Palette::for_context(ui.ctx());
        let nodes = &view.nodes;
        let scale = if self.settings.graph_uses_code_size {
            self.settings.code_size / 11.0
        } else {
            1.0
        };
        let font = super::code_fonts::family(ui.ctx(), &self.settings.code_font);
        let observed: BTreeSet<_> = graph
            .dynamic
            .iter()
            .flat_map(|trace| trace.blocks.iter().map(|block| block.start.as_str()))
            .collect();
        if !graph.dynamic.is_empty() {
            ui.label(format!(
                "{} blocks observed across {} workflow traces",
                observed.len(),
                graph.dynamic.len()
            ));
            ui.small("Highlighted blocks have runtime observations. Unobserved blocks may still execute in other runs.");
        }
        egui::ScrollArea::both()
            .id_salt("control-flow")
            .show_viewport(ui, |ui, viewport| {
                ui.set_min_size(view.size * scale);
                let origin = ui.min_rect().min.to_vec2();
                paint_edges(ui, view, scale, origin, &palette);
                for node in nodes {
                    let bounds = Rect::from_min_max(node.rect.min * scale, node.rect.max * scale);
                    if !viewport.intersects(bounds) {
                        continue;
                    }
                    let rect = bounds.translate(origin);
                    let block = &graph.blocks[node.block];
                    ui.painter().rect(
                        rect,
                        8,
                        palette.well,
                        if observed.contains(block.start.as_str()) {
                            Stroke::new(2.0, palette.accent)
                        } else {
                            Stroke::new(1.0, palette.line)
                        },
                        egui::StrokeKind::Inside,
                    );
                    ui.painter().text(
                        rect.min + Vec2::splat(12.0 * scale),
                        Align2::LEFT_TOP,
                        &block.start,
                        FontId::new(12.0 * scale, font.clone()),
                        palette.accent,
                    );
                    for (index, line) in block.lines.iter().enumerate() {
                        let end = line
                            .char_indices()
                            .nth(45)
                            .map_or(line.len(), |(index, _)| index);
                        let text = &line[..end];
                        ui.painter().text(
                            rect.min + Vec2::new(12.0, 34.0 + index as f32 * 16.0) * scale,
                            Align2::LEFT_TOP,
                            text,
                            FontId::new(11.0 * scale, font.clone()),
                            palette.text,
                        );
                    }
                    if ui
                        .interact(rect, ui.id().with(&block.start), Sense::click())
                        .double_clicked()
                    {
                        ui.ctx().copy_text(block.start.clone());
                    }
                }
            });
        let mut selected = dynamic_evidence(ui, graph);
        ui.horizontal_wrapped(|ui| {
            let callers = if graph.callers_deferred {
                "callers in References".to_owned()
            } else {
                format!("{} callers", graph.callers.len())
            };
            ui.label(format!(
                "{} blocks · {callers} · {} callees",
                graph.blocks.len(),
                graph.callees.len()
            ));
            for address in graph.callers.iter().chain(&graph.callees).take(20) {
                if ui.small_button(address).clicked() {
                    selected = Some(address.clone());
                    break;
                }
            }
        });
        if let Some(address) = selected {
            self.workspace.selected = address;
            self.select_view(crate::model::View::Graph);
        }
    }
}

fn dynamic_evidence(ui: &mut egui::Ui, graph: &Graph) -> Option<String> {
    let mut selected = None;
    for trace in &graph.dynamic {
        ui.push_id(&trace.trace_id, |ui| {
            let title = format!("Workflow: {} · {}", trace.workflow, trace.trace_id);
            ui.collapsing(title, |ui| {
                ui.label(format!(
                    "{:?} · {} · {}",
                    trace.source, trace.runtime_version, trace.captured_at
                ));
                let status = if trace.complete {
                    "Collector reports a normal end; observations cover only the selected workflow."
                } else {
                    "Partial capture or debugger sample; execution evidence is incomplete."
                };
                ui.label(status);
                ui.label(format!(
                    "{} observed blocks · {} observed call targets · {} unmapped observations",
                    trace.blocks.len(),
                    trace.calls.len(),
                    trace.unmapped_observations
                ));

                for call in trace.calls.iter().take(100) {
                    ui.horizontal(|ui| {
                        ui.label(format!("{} →", call.site));
                        if let Some(target) = &call.target {
                            if ui.small_button(target).clicked() {
                                selected = Some(target.clone());
                            }
                        } else {
                            ui.label(format!("{} (outside target module)", call.runtime_target));
                        }
                        ui.label(format!("· {} observations", call.observations));
                    });
                }
                if trace.calls.len() > 100 {
                    ui.small("Showing 100 call targets. Use execution_evidence to inspect the full trace.");
                }
            });
        });
    }
    selected
}

fn layout(graph: &Graph) -> Vec<Node> {
    let mut sorted: Vec<_> = graph.blocks.iter().enumerate().collect();
    sorted.sort_by_key(|(_, block)| address_value(&block.start).unwrap_or_default());
    let mut levels = BTreeMap::<&str, usize>::new();
    let mut rows = BTreeMap::<usize, Vec<usize>>::new();
    for (index, block) in sorted {
        let level = levels
            .get(block.start.as_str())
            .copied()
            .unwrap_or_default();
        rows.entry(level).or_default().push(index);
        for successor in &block.successors {
            if address_value(successor) > address_value(&block.start) {
                levels
                    .entry(successor)
                    .and_modify(|depth| *depth = (*depth).max(level + 1))
                    .or_insert(level + 1);
            }
        }
    }
    let mut nodes = Vec::with_capacity(graph.blocks.len());
    let mut y = 24.0;
    for row in rows.values() {
        let height = row
            .iter()
            .map(|index| 50.0 + graph.blocks[*index].lines.len() as f32 * 16.0)
            .fold(66.0, f32::max);
        for (column, index) in row.iter().enumerate() {
            nodes.push(Node {
                block: *index,
                rect: Rect::from_min_size(
                    Pos2::new(24.0 + column as f32 * 330.0, y),
                    Vec2::new(290.0, height),
                ),
            });
        }
        y += height + 56.0;
    }
    nodes
}

fn edge_indices(graph: &Graph, nodes: &[Node]) -> Vec<(usize, usize)> {
    let mut by_address = BTreeMap::new();
    for (index, node) in nodes.iter().enumerate() {
        by_address
            .entry(graph.blocks[node.block].start.as_str())
            .or_insert(index);
    }
    let mut edges = Vec::new();
    for (source, node) in nodes.iter().enumerate() {
        for successor in &graph.blocks[node.block].successors {
            if let Some(&target) = by_address.get(successor.as_str()) {
                edges.push((source, target));
            }
        }
    }
    edges
}

fn paint_edges(ui: &egui::Ui, view: &GraphView, scale: f32, origin: Vec2, palette: &Palette) {
    for &(source, target) in &view.edges {
        let source = &view.nodes[source];
        let target = &view.nodes[target];
        let start = source.rect.center_bottom() * scale + origin;
        let end = target.rect.center_top() * scale + origin;
        if !ui
            .clip_rect()
            .intersects(Rect::from_two_pos(start, end).expand(8.0))
        {
            continue;
        }
        let color = if target.rect.top() <= source.rect.top() {
            egui::Color32::from_rgb(198, 137, 69)
        } else {
            palette.accent
        };
        let middle = (start.y + end.y) * 0.5;
        let points = vec![
            start,
            Pos2::new(start.x, middle),
            Pos2::new(end.x, middle),
            end,
        ];
        ui.painter()
            .add(egui::Shape::line(points, Stroke::new(1.3, color)));
        ui.painter().arrow(
            end - Vec2::new(0.0, 7.0),
            Vec2::new(0.0, 7.0),
            Stroke::new(1.3, color),
        );
    }
}
