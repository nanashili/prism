use crate::ui::message::layout::LayoutKey;
use eframe::egui;

pub(super) struct RowSpec {
    pub id: egui::Id,
    pub estimate: f32,
}

struct Row {
    id: egui::Id,
    height: f32,
}

#[derive(Clone, Copy)]
pub(super) struct Anchor {
    id: egui::Id,
    within: f32,
}

#[derive(Default)]
pub(super) struct Rows {
    rows: Vec<Row>,
    scratch: Vec<Row>,
    layout: Option<LayoutKey>,
}

impl Rows {
    pub fn sync(&mut self, specs: &[RowSpec], layout: LayoutKey) {
        let reflow = self.layout != Some(layout);
        self.scratch.clear();
        for spec in specs {
            let previous = self.rows.iter().position(|row| row.id == spec.id);
            let row = previous.map(|index| self.rows.swap_remove(index));
            self.scratch.push(match row {
                Some(row) if !reflow => row,
                _ => Row {
                    id: spec.id,
                    height: spec.estimate,
                },
            });
        }
        std::mem::swap(&mut self.rows, &mut self.scratch);
        self.scratch.clear();
        self.layout = Some(layout);
    }

    pub fn anchor(&self, offset: f32) -> Option<Anchor> {
        let mut start = 0.0;
        for row in &self.rows {
            if start + row.height > offset {
                return Some(Anchor {
                    id: row.id,
                    within: offset - start,
                });
            }
            start += row.height;
        }
        None
    }

    pub fn position(&self, anchor: Anchor) -> Option<f32> {
        let mut start = 0.0;
        for row in &self.rows {
            if row.id == anchor.id {
                return Some(start + anchor.within.min(row.height));
            }
            start += row.height;
        }
        None
    }

    pub fn first_intersecting(&self, offset: f32) -> usize {
        let mut end = 0.0;
        self.rows
            .iter()
            .position(|row| {
                end += row.height;
                end >= offset
            })
            .unwrap_or(self.rows.len())
    }

    pub fn start(&self, index: usize) -> f32 {
        self.rows[..index].iter().map(|row| row.height).sum()
    }

    pub fn total(&self) -> f32 {
        self.rows.iter().map(|row| row.height).sum()
    }

    pub fn measure(&mut self, index: usize, height: f32) -> bool {
        let height = height.max(1.0);
        let changed = (self.rows[index].height - height).abs() > 0.5;
        self.rows[index].height = height;
        changed
    }
}
