use super::Database;
use crate::error::Result;
use serde_json::{Value, json};
use std::io::Write;

impl Database {
    pub fn report(&self, generation: i64) -> Result<Value> {
        let tx = self.db.unchecked_transaction()?;
        let status = self.status(generation)?;
        let path = self.root.join(".prism-reconstruction").join(format!(
            "generation-{generation}-revision-{}.json",
            status["generation"]["revision"]
        ));
        crate::storage::atomic::write(&path, |file| {
            file.write_all(b"{\"status\":")?;
            serde_json::to_writer(&mut *file, &status)?;
            file.write_all(b",\"units\":[")?;
            let mut query = tx.prepare(
                "SELECT address,name,state,reason FROM units WHERE generation=?1 ORDER BY address",
            )?;
            let mut rows = query.query([generation])?;
            let mut first = true;
            while let Some(row) = rows.next()? {
                if !first {
                    file.write_all(b",")?;
                }
                first = false;
                serde_json::to_writer(
                    &mut *file,
                    &json!({
                        "address":row.get::<_, String>(0)?,"name":row.get::<_, String>(1)?,
                        "state":row.get::<_, String>(2)?,"reason":row.get::<_, String>(3)?,
                    }),
                )?;
            }
            file.write_all(b"]}")?;
            Ok(())
        })?;
        tx.commit()?;
        Ok(json!({"report":path,"behavior_verified":false}))
    }
}
