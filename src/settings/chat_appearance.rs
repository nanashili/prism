use crate::error::Result;
use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct ChatAppearance {
    pub font: String,
    pub text_size: f32,
    pub code_size: f32,
    pub line_spacing: f32,
    pub paragraph_spacing: f32,
    pub message_spacing: f32,
    pub block_colors: bool,
}

impl Default for ChatAppearance {
    fn default() -> Self {
        Self {
            font: "System".into(),
            text_size: 15.0,
            code_size: 13.0,
            line_spacing: 150.0,
            paragraph_spacing: 10.0,
            message_spacing: 24.0,
            block_colors: true,
        }
    }
}

impl ChatAppearance {
    pub fn validate(&self) -> Result<()> {
        if !["System", "Monospace"].contains(&self.font.as_str()) {
            return Err("The chat font is outside its supported range".into());
        }
        for (value, range) in [
            (self.text_size, 12.0..=22.0),
            (self.code_size, 11.0..=20.0),
            (self.line_spacing, 120.0..=190.0),
            (self.paragraph_spacing, 4.0..=24.0),
            (self.message_spacing, 12.0..=48.0),
        ] {
            if !range.contains(&value) {
                return Err("A chat appearance setting is outside its supported range".into());
            }
        }
        Ok(())
    }

    pub fn line_height(&self, size: f32) -> f32 {
        size * self.line_spacing / 100.0
    }
}
