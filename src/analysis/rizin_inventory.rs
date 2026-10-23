use super::rizin::Rizin;
use crate::error::Result;
use crate::model::Procedure;
use crate::storage::Store;

impl Rizin {
    pub fn index(&mut self, store: &mut Store, project: &str) -> Result<usize> {
        self.analyze()?;
        let reply = self.command("afl")?;
        let text = reply
            .as_str()
            .ok_or("Rizin did not return its function list")?;
        let mut inventory = store.begin_functions(project)?;
        let mut page = Vec::with_capacity(256);
        for line in text.lines().filter(|line| !line.trim().is_empty()) {
            page.push(parse(line)?);
            if page.len() == 256 {
                inventory.append(&page)?;
                page.clear();
            }
        }
        inventory.append(&page)?;
        inventory.finish()
    }
}

pub(super) fn parse(line: &str) -> Result<Procedure> {
    let (address, tail) = field(line)?;
    let (_, tail) = field(tail)?;
    let (_, mut name) = field(tail)?;
    if let Some(tail) = name.strip_prefix("->") {
        name = field(tail)?.1;
    }
    if name.is_empty() {
        return Err("A Rizin function has no name".into());
    }
    let address =
        super::graph::address_value(address).ok_or("A Rizin function has an invalid address")?;
    Ok(Procedure {
        address: format!("0x{address:x}"),
        name: name.into(),
        kind: if name.starts_with("sym.imp.") {
            "Thunk".into()
        } else {
            "Function".into()
        },
    })
}

fn field(text: &str) -> Result<(&str, &str)> {
    let text = text.trim_start();
    let boundary = text
        .find(char::is_whitespace)
        .ok_or("The Rizin function list has an unsupported row format")?;
    Ok((&text[..boundary], text[boundary..].trim_start()))
}
