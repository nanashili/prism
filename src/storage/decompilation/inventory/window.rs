use crate::error::{Result, failure};
use crate::storage::Store;
use rusqlite::params;

impl Store {
    pub(crate) fn decompilation_missing_window(
        &self,
        id: &str,
        offset: usize,
        limit: usize,
    ) -> Result<Vec<(usize, String)>> {
        if !(1..=256).contains(&limit) {
            return Err(failure(
                "A durable decompilation window requires between 1 and 256 addresses",
            ));
        }

        let mut query = self.db.prepare(
            "SELECT i.position,i.address FROM decompilation_items i
             LEFT JOIN decompilation_results r ON r.job=i.job AND r.address=i.address
             WHERE i.job=?1 AND i.position>=?2
               AND (i.position=?2 OR coalesce(r.status,'')<>'succeeded')
             ORDER BY i.position LIMIT ?3",
        )?;
        let rows = query.query_map(
            params![id, i64::try_from(offset)?, i64::try_from(limit)?],
            |row| {
                let position: i64 = row.get(0)?;
                let position = usize::try_from(position)
                    .map_err(|_| rusqlite::Error::IntegralValueOutOfRange(0, position))?;
                Ok((position, row.get(1)?))
            },
        )?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }
}
