use std::ops::Range;

#[cfg(test)]
mod tests;

pub(in crate::ui) enum Lines {
    Compact(Vec<u32>),
    Wide(Vec<usize>),
}

impl Default for Lines {
    fn default() -> Self {
        Self::Compact(Vec::new())
    }
}

impl Lines {
    pub(in crate::ui) fn new(text: &str) -> Self {
        if u32::try_from(text.len()).is_ok() {
            Self::Compact(starts(text).map(|start| start as u32).collect())
        } else {
            // Keep native offsets for unusually large tool output rather than
            // truncating addresses or placing a new limit on accepted content.
            Self::Wide(starts(text).collect())
        }
    }

    pub(in crate::ui) fn len(&self) -> usize {
        match self {
            Self::Compact(starts) => starts.len(),
            Self::Wide(starts) => starts.len(),
        }
    }

    pub(in crate::ui) fn range(&self, index: usize, text_len: usize) -> Range<usize> {
        match self {
            Self::Compact(starts) => {
                starts[index] as usize..starts.get(index + 1).map_or(text_len, |end| *end as usize)
            }
            Self::Wide(starts) => starts[index]..starts.get(index + 1).copied().unwrap_or(text_len),
        }
    }
}

fn starts(text: &str) -> impl Iterator<Item = usize> + '_ {
    let mut start = 0;
    text.split_inclusive('\n').map(move |line| {
        let previous = start;
        start += line.len();
        previous
    })
}
