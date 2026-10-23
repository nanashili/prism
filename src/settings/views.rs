use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct Views {
    pub first_view: String,
    pub universal_apps: String,
    pub address_prefix: bool,
    pub address_upper: bool,
    pub address_pad: bool,
    pub hex_row: String,
    pub list_limit: String,
    pub minimap: bool,
    pub block_lines: String,
    pub keymap: String,
    pub shortcuts: std::collections::BTreeMap<String, String>,
}

impl Default for Views {
    fn default() -> Self {
        Self {
            first_view: "Pseudocode".into(),
            universal_apps: "Open the one for this Mac".into(),
            address_prefix: true,
            address_upper: false,
            address_pad: false,
            hex_row: "16".into(),
            list_limit: "1,000".into(),
            minimap: false,
            block_lines: "All".into(),
            keymap: "Prism".into(),
            shortcuts: Default::default(),
        }
    }
}

impl Views {
    pub fn address(&self, address: &str) -> String {
        let digits = address.trim_start_matches("0x").trim_start_matches("0X");
        let Ok(value) = u64::from_str_radix(digits, 16) else {
            return address.into();
        };
        let width = if self.address_pad { 16 } else { 0 };
        let digits = if self.address_upper {
            format!("{value:0width$X}")
        } else {
            format!("{value:0width$x}")
        };
        format!("{}{digits}", if self.address_prefix { "0x" } else { "" })
    }
}
