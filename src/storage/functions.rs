use super::Store;
use crate::error::{Result, failure};
use crate::model::Procedure;
use rusqlite::{Transaction, params};

pub(crate) struct FunctionImport<'a> {
    transaction: Transaction<'a>,
    project: String,
    count: usize,
}

impl Store {
    pub fn has_function(&self, project: &str, address: &str) -> Result<bool> {
        Ok(self
            .db
            .prepare_cached(
                "SELECT EXISTS(SELECT 1 FROM functions WHERE project=?1 AND address=?2)",
            )?
            .query_row(params![project, address], |row| row.get(0))?)
    }

    pub fn has_analysis(&self, project: &str) -> Result<bool> {
        Ok(self.db.query_row(
            "SELECT EXISTS(SELECT 1 FROM analysis_runs WHERE project=?1)
                 OR EXISTS(SELECT 1 FROM functions WHERE project=?1)",
            [project],
            |row| row.get(0),
        )?)
    }

    pub fn begin_functions(&mut self, project: &str) -> Result<FunctionImport<'_>> {
        let transaction = self.db.transaction()?;
        transaction.execute("DELETE FROM functions WHERE project=?1", [project])?;
        transaction.execute("DELETE FROM function_sources WHERE project=?1", [project])?;
        Ok(FunctionImport {
            transaction,
            project: project.into(),
            count: 0,
        })
    }
}

impl FunctionImport<'_> {
    pub fn source(&self, address: &str, path: &str, owner: &str, signature: &str) -> Result<()> {
        self.transaction
            .prepare_cached("INSERT INTO function_sources VALUES(?1,?2,?3,?4,?5)")?
            .execute(params![self.project, address, path, owner, signature])?;
        Ok(())
    }

    pub fn append(&mut self, functions: &[Procedure]) -> Result<()> {
        // Providers may repeat an address within a page or across pages. Keep
        // its first name and let unrelated database failures propagate.
        let mut insert = self.transaction.prepare_cached(
            "INSERT INTO functions(project,address,name,kind) VALUES(?1,?2,?3,?4)
             ON CONFLICT(project,address) DO NOTHING",
        )?;
        for function in functions {
            self.count += insert.execute(params![
                self.project,
                function.address,
                function.name,
                function.kind
            ])?;
            if self.count > 1_000_000 {
                return Err(failure("The function inventory exceeds one million rows"));
            }
        }
        Ok(())
    }

    pub fn finish(self) -> Result<usize> {
        self.transaction
            .execute_batch("DROP TABLE IF EXISTS temp.sidebar_identity")?;
        self.transaction.execute(
            "INSERT INTO analysis_runs VALUES(?1,unixepoch())
             ON CONFLICT(project) DO UPDATE SET completed=unixepoch()",
            [&self.project],
        )?;
        self.transaction.commit()?;
        Ok(self.count)
    }
}

impl Store {
    pub fn find_function(&self, project: &str, reference: &str) -> Result<Vec<String>> {
        let mut query = self.db.prepare_cached(
            "SELECT address FROM functions WHERE project=?1 AND (address=?2 OR name=?2) LIMIT 2",
        )?;
        let rows = query.query_map(params![project, reference], |row| row.get(0))?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    pub fn function_source(
        &self,
        project: &str,
        address: &str,
    ) -> Result<(String, String, String)> {
        Ok(self.db.query_row(
            "SELECT path,owner,signature FROM function_sources WHERE project=?1 AND address=?2",
            params![project, address],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )?)
    }
}
