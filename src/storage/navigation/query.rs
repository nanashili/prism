use super::{Page, Query, Row, Show, Sort};
use crate::error::Result;
use crate::model::Procedure;
use crate::storage::Store;
use rusqlite::params;

const SOURCE: &str = "FROM sidebar_functions n
    LEFT JOIN sidebar_renames r ON r.address=n.address";
const VISIBLE: &str = "(?3=0 OR (?3=1 AND n.origin='Your code')
    OR (?3=2 AND r.address IS NOT NULL) OR (?3=3 AND n.origin='Imports'))";
const MATCHES: &str = "(?2='' OR instr(lower(coalesce(r.name,f.name)),lower(?2))>0
        OR instr(lower(f.address),lower(?2))>0 OR instr(lower(n.file),lower(?2))>0
        OR (?4=1 AND instr(lower(coalesce(nullif(n.module,''),
            CASE WHEN n.origin='Imports' THEN 'Imports' ELSE 'Unmapped' END)),lower(?2))>0))";

impl Store {
    pub(crate) fn navigation_page(
        &mut self,
        project: &str,
        query: &Query,
        cancelled: &std::sync::atomic::AtomicBool,
    ) -> Result<Page> {
        self.prepare_navigation(project, cancelled)?;
        self.navigation_renames(query)?;
        let show = match query.show {
            Show::All => 0,
            Show::Yours => 1,
            Show::Renamed => 2,
            Show::Imports => 3,
        };
        let modules = i32::from(query.sort == Sort::Modules);
        let total = self
            .db
            .query_row("SELECT count(*) FROM sidebar_functions", [], |row| {
                row.get::<_, u32>(0).map(|value| value as usize)
            })?;
        let matched = self.navigation_matched(project, query, show, modules, total)?;
        let grouped =
            query.filter.trim().is_empty() && matches!(query.sort, Sort::Modules | Sort::Files);
        let mut rows = if grouped
            && (query.parent.is_empty()
                || (query.sort == Sort::Files && query.parent.ends_with('/')))
        {
            self.navigation_groups(project, query, show, modules)?
        } else {
            self.navigation_functions(project, query, show, modules)?
        };
        let more = rows.len() > 300;
        rows.truncate(300);
        Ok(Page {
            rows,
            total,
            matched,
            counts: Default::default(),
            more,
        })
    }

    fn navigation_matched(
        &self,
        project: &str,
        query: &Query,
        show: i32,
        modules: i32,
        total: usize,
    ) -> Result<usize> {
        if query.filter.trim().is_empty() {
            if query.show == Show::All {
                return Ok(total);
            }
            return Ok(self.db.query_row(
                &format!("SELECT count(*) {SOURCE} WHERE {VISIBLE}"),
                params![project, "", show],
                |row| row.get::<_, u32>(0).map(|value| value as usize),
            )?);
        }
        Ok(self.db.query_row(
            &format!(
                "SELECT count(*) {SOURCE}
                 JOIN functions f ON f.project=?1 AND f.address=n.address
                 WHERE {VISIBLE} AND {MATCHES}"
            ),
            params![project, query.filter.trim(), show, modules],
            |row| row.get::<_, u32>(0).map(|value| value as usize),
        )?)
    }

    fn navigation_renames(&mut self, query: &Query) -> Result<()> {
        let transaction = self.db.transaction()?;
        transaction.execute("DELETE FROM sidebar_renames", [])?;
        {
            let mut insert = transaction.prepare("INSERT INTO sidebar_renames VALUES(?1,?2)")?;
            for (address, name) in &query.renamed {
                insert.execute(params![address, name])?;
            }
        }
        transaction.commit()?;
        Ok(())
    }

    fn navigation_groups(
        &self,
        project: &str,
        query: &Query,
        show: i32,
        modules: i32,
    ) -> Result<Vec<Row>> {
        let group = if modules == 1 {
            "coalesce(nullif(n.module,''),CASE WHEN n.origin='Imports' THEN 'Imports' ELSE 'Unmapped' END)"
        } else {
            "n.file"
        };
        let prefix = &query.parent;
        let sql = format!(
            "WITH matched AS (SELECT {group} AS path {SOURCE} WHERE {VISIBLE})
             SELECT CASE WHEN ?4=1 THEN path ELSE
                ?5 || CASE WHEN instr(substr(path,length(?5)+1),'/')>0
                  THEN substr(substr(path,length(?5)+1),1,instr(substr(path,length(?5)+1),'/'))
                  ELSE substr(path,length(?5)+1) END END AS key,count(*)
             FROM matched WHERE substr(path,1,length(?5))=?5
             GROUP BY key ORDER BY key COLLATE NOCASE LIMIT 301 OFFSET ?6"
        );
        let mut statement = self.db.prepare(&sql)?;
        let rows = statement.query_map(
            params![
                project,
                "",
                show,
                modules,
                prefix,
                i64::try_from(query.offset)?
            ],
            |row| {
                let key: String = row.get(0)?;
                let name = key
                    .trim_end_matches('/')
                    .rsplit('/')
                    .next()
                    .unwrap_or(&key)
                    .to_owned();
                Ok(Row::Group {
                    key,
                    name,
                    count: row.get::<_, u32>(1)? as usize,
                })
            },
        )?;
        let groups: Vec<Row> = rows.collect::<std::result::Result<_, _>>()?;
        // A source leaf owns its functions rather than another identical folder row.
        if modules == 0
            && groups.len() == 1
            && matches!(&groups[0], Row::Group { key, .. } if key == prefix || key.trim_end_matches('/') == prefix.trim_end_matches('/'))
        {
            return self.navigation_functions(project, query, show, modules);
        }
        Ok(groups)
    }

    fn navigation_functions(
        &self,
        project: &str,
        query: &Query,
        show: i32,
        modules: i32,
    ) -> Result<Vec<Row>> {
        let order = if query.sort == Sort::Address {
            "n.position"
        } else {
            "coalesce(r.name,f.name) COLLATE NOCASE"
        };
        let sql = format!(
            "SELECT f.address,coalesce(r.name,f.name),f.kind,n.module,n.file,n.inferred
             {SOURCE} JOIN functions f ON f.project=?1 AND f.address=n.address
             WHERE {VISIBLE} AND {MATCHES}
             AND (?5='' OR (?4=1 AND coalesce(nullif(n.module,''),
                CASE WHEN n.origin='Imports' THEN 'Imports' ELSE 'Unmapped' END)=?5)
                OR (?4=0 AND (n.file=?5 OR n.file=rtrim(?5,'/'))))
             ORDER BY {order} LIMIT 301 OFFSET ?6"
        );
        let mut statement = self.db.prepare(&sql)?;
        let rows = statement.query_map(
            params![
                project,
                query.filter.trim(),
                show,
                modules,
                query.parent,
                i64::try_from(query.offset)?
            ],
            |row| {
                let module: String = row.get(3)?;
                let file: String = row.get(4)?;
                Ok(Row::Function {
                    procedure: Procedure {
                        address: row.get(0)?,
                        name: row.get(1)?,
                        kind: row.get(2)?,
                    },
                    location: if module.is_empty() {
                        file
                    } else {
                        format!("{module} · {file}")
                    },
                    inferred: row.get(5)?,
                })
            },
        )?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }
}
