//! Persistent, configuration-scoped evidence and native structural postings for function search.

#[cfg(test)]
mod tests;

use crate::analysis::semantic::{Document, Hit, Query, StructuralMatch};
use crate::error::{Result, failure};
use crate::storage::Store;
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};

/// Bound reranking work independently of the size of an indexed corpus.
pub(crate) const CANDIDATE_LIMIT: usize = 400;
const INDEX_BUDGET: i64 = 256 * 1024 * 1024;

/// A retrieval request tied to one binary and analysis configuration.
#[derive(Debug)]
pub(crate) struct SearchRequest<'a> {
    pub scope: &'a str,
    pub project: &'a str,
    pub query: &'a Query,
    pub limit: usize,
    pub similar_to: Option<&'a str>,
    pub cancelled: &'a AtomicBool,
}

#[derive(Debug, serde::Serialize)]
pub(crate) struct Coverage {
    pub indexed_functions: usize,
    pub inventory_functions: usize,
}

pub(super) fn initialize(db: &Connection) -> Result<()> {
    db.execute_batch(
        "CREATE TABLE IF NOT EXISTS function_search_documents(
            id INTEGER PRIMARY KEY,scope TEXT NOT NULL,address TEXT NOT NULL,
            document TEXT NOT NULL,UNIQUE(scope,address));
         CREATE VIRTUAL TABLE IF NOT EXISTS function_search_terms
            USING fts5(terms, tokenize='unicode61');
         CREATE TABLE IF NOT EXISTS function_search_features(
            scope TEXT NOT NULL,address TEXT NOT NULL,space TEXT NOT NULL,
            hash INTEGER NOT NULL,weight REAL NOT NULL,
            PRIMARY KEY(scope,address,hash));
         CREATE INDEX IF NOT EXISTS function_search_feature_postings
            ON function_search_features(scope,space,hash);",
    )?;
    Ok(())
}

impl Store {
    pub(crate) fn search_document(&self, scope: &str, address: &str) -> Result<Option<Document>> {
        let raw: Option<String> = self
            .db
            .query_row(
                "SELECT document FROM function_search_documents WHERE scope=?1 AND address=?2",
                params![scope, address],
                |row| row.get(0),
            )
            .optional()?;
        raw.map(|raw| serde_json::from_str(&raw).map_err(Into::into))
            .transpose()
    }

    pub(crate) fn index_search_documents(
        &self,
        scope: &str,
        documents: &[Document],
        cancelled: &AtomicBool,
    ) -> Result<()> {
        let transaction = self.db.unchecked_transaction()?;
        for document in documents {
            check_cancelled(cancelled)?;
            write_document(&transaction, scope, document)?;
        }
        enforce_budget(&transaction)?;
        check_cancelled(cancelled)?;
        transaction.commit()?;
        Ok(())
    }

    /// Remove derived indexes for this binary, including obsolete configuration scopes.
    pub(crate) fn clear_function_search(
        &self,
        project: &str,
        cancelled: &AtomicBool,
    ) -> Result<usize> {
        check_cancelled(cancelled)?;
        let prefix = format!("function-search-v1:{project}:%");
        let transaction = self.db.unchecked_transaction()?;
        transaction.execute(
            "DELETE FROM function_search_features WHERE scope LIKE ?1",
            [&prefix],
        )?;
        transaction.execute(
            "DELETE FROM function_search_terms WHERE rowid IN
             (SELECT id FROM function_search_documents WHERE scope LIKE ?1)",
            [&prefix],
        )?;
        let removed = transaction.execute(
            "DELETE FROM function_search_documents WHERE scope LIKE ?1",
            [&prefix],
        )?;
        check_cancelled(cancelled)?;
        transaction.commit()?;
        Ok(removed)
    }

    pub(crate) fn function_search_coverage(&self, scope: &str, project: &str) -> Result<Coverage> {
        let (indexed, inventory): (u32, u32) = self.db.query_row(
            "SELECT count(*),(SELECT count(*) FROM functions WHERE project=?2)
             FROM function_search_documents d JOIN functions f
               ON f.project=?2 AND f.address=d.address WHERE d.scope=?1",
            params![scope, project],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        Ok(Coverage {
            indexed_functions: usize::try_from(indexed)?,
            inventory_functions: usize::try_from(inventory)?,
        })
    }

    pub(crate) fn search_functions(&self, request: &SearchRequest<'_>) -> Result<Vec<Hit>> {
        let SearchRequest {
            scope,
            project,
            query,
            limit,
            cancelled,
            ..
        } = *request;
        let mut hits = self.text_candidates(scope, project, query, cancelled)?;
        let seeds = self.structure_seeds(request, &hits)?;

        for (address, relevance) in seeds {
            check_cancelled(cancelled)?;
            self.expand_structure(request, &address, relevance, &mut hits)?;
        }
        let mut ranked: Vec<_> = hits.into_values().collect();
        ranked.sort_by(|left, right| {
            right
                .score
                .total_cmp(&left.score)
                .then(left.address.cmp(&right.address))
        });
        ranked.truncate(limit);
        Ok(ranked)
    }

    fn structure_seeds(
        &self,
        request: &SearchRequest<'_>,
        hits: &BTreeMap<String, Hit>,
    ) -> Result<Vec<(String, f64)>> {
        let SearchRequest {
            scope,
            project,
            similar_to,
            ..
        } = *request;
        let seeds = if let Some(address) = similar_to {
            if !self.has_function(project, address)? {
                return Err(failure(
                    "The exemplar is not in the saved function inventory",
                ));
            }
            let seed = self
                .search_document(scope, address)?
                .ok_or("Index the exemplar before searching for similar functions")?;
            if seed.structure.is_empty() {
                return Err(failure("The exemplar has no native structural signature"));
            }
            vec![(address.into(), 1.0)]
        } else {
            let mut ranked: Vec<_> = hits.values().collect();
            sort_hits(&mut ranked);
            ranked
                .into_iter()
                .filter(|hit| hit.score >= 0.2)
                .take(3)
                .map(|hit| (hit.address.clone(), hit.score))
                .collect()
        };

        Ok(seeds)
    }

    fn text_candidates(
        &self,
        scope: &str,
        project: &str,
        query: &Query,
        cancelled: &AtomicBool,
    ) -> Result<BTreeMap<String, Hit>> {
        let mut statement = self.db.prepare(
            "SELECT d.document FROM function_search_terms t
             JOIN function_search_documents d ON d.id=t.rowid
             JOIN functions f ON f.project=?2 AND f.address=d.address
             WHERE function_search_terms MATCH ?3 AND d.scope=?1
             ORDER BY bm25(function_search_terms),d.address LIMIT ?4",
        )?;
        let rows = statement.query_map(
            params![scope, project, query.fts(), i64::try_from(CANDIDATE_LIMIT)?],
            |row| row.get::<_, String>(0),
        )?;
        let mut hits = BTreeMap::new();
        for row in rows {
            check_cancelled(cancelled)?;
            let document: Document = serde_json::from_str(&row?)?;
            let hit = document.rank(query);
            if hit.score > 0.0 {
                hits.insert(hit.address.clone(), hit);
            }
        }
        Ok(hits)
    }

    fn expand_structure(
        &self,
        request: &SearchRequest<'_>,
        address: &str,
        relevance: f64,
        hits: &mut BTreeMap<String, Hit>,
    ) -> Result<()> {
        let SearchRequest {
            scope,
            project,
            query,
            cancelled,
            ..
        } = *request;
        let mut statement = self.db.prepare(
            "SELECT d.document,sum(a.weight*b.weight) AS cosine,count(*)
             FROM function_search_features a JOIN function_search_features b
               ON b.scope=a.scope AND b.space=a.space AND b.hash=a.hash
             JOIN function_search_documents d ON d.scope=b.scope AND d.address=b.address
             JOIN functions f ON f.project=?3 AND f.address=b.address
             WHERE a.scope=?1 AND a.address=?2 AND b.address<>a.address
             GROUP BY b.address HAVING cosine>=0.85 AND count(*)>=3
             ORDER BY cosine DESC,b.address LIMIT ?4",
        )?;
        let rows = statement.query_map(
            params![scope, address, project, i64::try_from(CANDIDATE_LIMIT)?],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, f64>(1)?,
                    row.get::<_, u32>(2)?,
                ))
            },
        )?;
        for row in rows {
            check_cancelled(cancelled)?;
            let (raw, cosine, shared) = row?;
            let document: Document = serde_json::from_str(&raw)?;
            let hit = hits
                .entry(document.address.clone())
                .or_insert_with(|| document.rank(query));
            let structural_score = 0.8 * relevance * cosine.clamp(0.0, 1.0);
            // Retain the strongest independent retrieval route, rather than counting
            // correlated evidence twice or promoting several copies of one signature.
            if hit
                .structural
                .as_ref()
                .is_none_or(|previous| structural_score > previous.score)
            {
                hit.score = hit.score.max(structural_score);
                hit.structural = Some(StructuralMatch {
                    exemplar: address.into(),
                    seed_relevance: relevance,
                    score: structural_score,
                    cosine: cosine.clamp(0.0, 1.0),
                    shared_features: usize::try_from(shared)?,
                });
            }
        }
        Ok(())
    }
}

fn sort_hits(hits: &mut [&Hit]) {
    hits.sort_by(|left, right| {
        right
            .score
            .total_cmp(&left.score)
            .then(left.address.cmp(&right.address))
    });
}

fn write_document(transaction: &Transaction<'_>, scope: &str, document: &Document) -> Result<()> {
    document.validate()?;
    let raw = serde_json::to_string(document)?;
    let id: i64 = transaction.query_row(
        "INSERT INTO function_search_documents(scope,address,document) VALUES(?1,?2,?3)
         ON CONFLICT(scope,address) DO UPDATE SET document=excluded.document RETURNING id",
        params![scope, document.address, raw],
        |row| row.get(0),
    )?;
    replace_terms(transaction, id, &document.index_terms())?;
    replace_structure(transaction, scope, document)
}

fn replace_terms(transaction: &Transaction<'_>, id: i64, terms: &str) -> Result<()> {
    transaction.execute("DELETE FROM function_search_terms WHERE rowid=?1", [id])?;
    transaction.execute(
        "INSERT INTO function_search_terms(rowid,terms) VALUES(?1,?2)",
        params![id, terms],
    )?;
    Ok(())
}

fn replace_structure(
    transaction: &Transaction<'_>,
    scope: &str,
    document: &Document,
) -> Result<()> {
    transaction.execute(
        "DELETE FROM function_search_features WHERE scope=?1 AND address=?2",
        params![scope, document.address],
    )?;

    let norm = document
        .structure
        .iter()
        .fold(0.0_f64, |norm, feature| norm.hypot(feature.weight));
    if !norm.is_finite() {
        return Err(failure("The native structural vector norm overflowed"));
    }
    let mut insert = transaction
        .prepare_cached("INSERT INTO function_search_features VALUES(?1,?2,?3,?4,?5)")?;
    for feature in &document.structure {
        insert.execute(params![
            scope,
            document.address,
            document.structure_space,
            feature.hash,
            feature.weight / norm
        ])?;
    }
    Ok(())
}

fn enforce_budget(transaction: &Transaction<'_>) -> Result<()> {
    let bytes: i64 = transaction.query_row(
        "SELECT coalesce(sum(length(CAST(document AS BLOB))),0)
           +(SELECT count(*)*64 FROM function_search_features) FROM function_search_documents",
        [],
        |row| row.get(0),
    )?;
    if bytes > INDEX_BUDGET {
        return Err(failure(
            "The function search index exceeds its 256 MiB evidence budget",
        ));
    }
    Ok(())
}

fn check_cancelled(cancelled: &AtomicBool) -> Result<()> {
    if cancelled.load(Ordering::Relaxed) {
        return Err(failure("Function search cancelled"));
    }
    Ok(())
}
