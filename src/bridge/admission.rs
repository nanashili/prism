use crate::error::{Result, failure};
use prism::mcp::CallClass;
use std::sync::{Arc, Mutex};

#[derive(Clone, Default)]
pub(super) struct Admission {
    usage: Arc<Mutex<[Usage; 4]>>,
}

#[derive(Clone, Copy, Default)]
struct Usage {
    requests: usize,
    bytes: usize,
}

pub(super) struct Permit {
    admission: Admission,
    class: CallClass,
    bytes: usize,
}

impl Admission {
    #[cfg_attr(feature = "profiling", hotpath::measure)]
    pub(super) fn acquire(&self, class: CallClass, bytes: usize) -> Result<Permit> {
        let mut classes = self
            .usage
            .lock()
            .map_err(|_| failure("Bridge admission is unavailable"))?;
        let usage = &mut classes[index(class)];
        if usage.requests >= class.capacity()
            || bytes > class.byte_budget().saturating_sub(usage.bytes)
        {
            return Err(failure(
                "Prism has too many pending requests in this tool class. Try again shortly.",
            ));
        }
        usage.requests += 1;
        usage.bytes += bytes;
        Ok(Permit {
            admission: self.clone(),
            class,
            bytes,
        })
    }
}

impl Drop for Permit {
    fn drop(&mut self) {
        match self.admission.usage.lock() {
            Ok(mut classes) => {
                let usage = &mut classes[index(self.class)];
                usage.requests -= 1;
                usage.bytes -= self.bytes;
            }
            Err(_) => eprintln!("Releasing bridge admission failed"),
        }
    }
}

fn index(class: CallClass) -> usize {
    match class {
        CallClass::Ordinary => 0,
        CallClass::Question => 1,
        CallClass::Control => 2,
        CallClass::Snapshot => 3,
    }
}
