use super::batch_item::item;
use super::batch_state::{Counts, DecompilationBatch, DecompilationItem};
use crate::error::{Result, failure};
use serde_json::Value;
use std::collections::HashSet;

pub(super) fn summary(value: &Value, batch: &mut DecompilationBatch) -> Result<()> {
    if value["kind"] != "summary" || value["parent_operation"] != "batch_decompile" {
        return Err(failure("REA omitted the retained decompilation summary"));
    }
    let evidence = value["parent_evidence_id"]
        .as_str()
        .filter(|id| !id.is_empty() && id.len() <= 1024)
        .ok_or("REA omitted the batch evidence identity")?;
    if batch
        .evidence
        .as_deref()
        .is_some_and(|expected| expected != evidence)
    {
        return Err(failure("REA returned a summary for a different batch"));
    }
    let total = number(&value["summary"]["total"])?;
    let succeeded = number(&value["summary"]["succeeded"])?;
    let failed = number(&value["summary"]["failed"])?;
    if total != batch.addresses.len() || succeeded.checked_add(failed) != Some(total) {
        return Err(failure(
            "REA returned inconsistent decompilation summary counts",
        ));
    }
    batch.evidence = Some(evidence.into());
    batch.expected = Counts { succeeded, failed };
    Ok(())
}

pub(super) fn page(
    mut value: Value,
    batch: &mut DecompilationBatch,
) -> Result<Vec<DecompilationItem>> {
    if value["kind"] != "page"
        || value["parent_operation"] != "batch_decompile"
        || value["parent_evidence_id"].as_str() != batch.evidence.as_deref()
        || number(&value["coverage"]["total"])? != batch.addresses.len()
    {
        return Err(failure(
            "REA returned decompilation evidence for a different batch",
        ));
    }
    let rows = value
        .get_mut("items")
        .and_then(Value::as_array_mut)
        .ok_or("REA omitted the decompilation page items")?;
    if rows.is_empty() || rows.len() > batch.page_limit {
        return Err(failure(
            "REA returned an empty or oversized decompilation page",
        ));
    }
    let end = batch
        .offset
        .checked_add(rows.len())
        .ok_or("Decompilation offset overflow")?;
    if end > batch.addresses.len() {
        return Err(failure("REA returned too many decompilation results"));
    }
    let mut counts = Counts::default();
    let items = rows
        .drain(..)
        .zip(&batch.addresses[batch.offset..end])
        .map(|(row, expected)| item(row, expected, &mut counts))
        .collect::<Result<Vec<_>>>()?;
    validate_coverage(&value["coverage"], batch, end, items.len())?;
    let succeeded = batch.observed.succeeded + counts.succeeded;
    let failed = batch.observed.failed + counts.failed;
    if succeeded > batch.expected.succeeded
        || failed > batch.expected.failed
        || (end == batch.addresses.len()
            && (succeeded != batch.expected.succeeded || failed != batch.expected.failed))
    {
        return Err(failure(
            "REA pages disagree with the decompilation summary counts",
        ));
    }
    batch.observed = Counts { succeeded, failed };
    batch.offset = end;
    Ok(items)
}

pub(super) fn rust_page(
    mut value: Value,
    batch: &mut DecompilationBatch,
    end: usize,
) -> Result<Vec<DecompilationItem>> {
    validate_rust_fields(&value)?;
    let addresses = batch
        .addresses
        .get(batch.offset..end)
        .filter(|addresses| !addresses.is_empty())
        .ok_or("Invalid saved Rust decompilation chunk")?;
    let total = number(&value["total"])?;
    let expected = Counts {
        succeeded: number(&value["succeeded"])?,
        failed: number(&value["failed"])?,
    };
    if total != addresses.len() || expected.succeeded.checked_add(expected.failed) != Some(total) {
        return Err(failure("REA returned inconsistent Rust batch counts"));
    }
    let rows = value
        .get_mut("items")
        .and_then(Value::as_array_mut)
        .filter(|rows| rows.len() == total)
        .ok_or("REA omitted or changed the Rust batch result count")?;

    let mut observed = Counts::default();
    let items = rows
        .drain(..)
        .zip(addresses)
        .map(|(row, expected)| item(row, expected, &mut observed))
        .collect::<Result<Vec<_>>>()?;
    if observed != expected {
        return Err(failure("REA Rust batch statuses disagree with its counts"));
    }
    validate_rust_durability(&value["durability"], addresses, &items)?;

    batch.offset = end;
    Ok(items)
}

fn validate_rust_fields(value: &Value) -> Result<()> {
    let Some(fields) = value.as_object() else {
        return Err(failure("REA returned an invalid Rust batch object"));
    };
    let expected = ["items", "total", "succeeded", "failed", "durability"];
    if fields.len() == expected.len()
        && fields
            .keys()
            .all(|field| expected.contains(&field.as_str()))
    {
        Ok(())
    } else {
        Err(failure("REA returned unexpected Rust batch fields"))
    }
}

fn validate_rust_durability(
    value: &Value,
    addresses: &[String],
    items: &[DecompilationItem],
) -> Result<()> {
    let Some(fields) = value.as_object() else {
        return Err(failure("REA omitted Rust batch durability receipts"));
    };
    if fields.len() != 2
        || !fields.contains_key("confirmed_addresses")
        || !fields.contains_key("unconfirmed_addresses")
    {
        return Err(failure(
            "REA returned malformed Rust batch durability receipts",
        ));
    }

    let confirmed = numeric_address_set(&value["confirmed_addresses"])?;
    let unconfirmed = numeric_address_set(&value["unconfirmed_addresses"])?;
    let selected = addresses
        .iter()
        .map(|address| crate::analysis::graph::address_value(address))
        .collect::<Option<HashSet<_>>>()
        .ok_or("Invalid saved Rust decompilation address")?;
    let successful = items
        .iter()
        .filter(|item| item.result.is_ok())
        .map(|item| crate::analysis::graph::address_value(&item.address))
        .collect::<Option<HashSet<_>>>()
        .ok_or("Invalid successful Rust decompilation address")?;
    if !unconfirmed.is_empty()
        || confirmed.iter().any(|address| !selected.contains(address))
        || successful
            .iter()
            .any(|address| !confirmed.contains(address))
    {
        return Err(failure(
            "REA returned inconsistent Rust batch durability receipts",
        ));
    }
    Ok(())
}

fn numeric_address_set(value: &Value) -> Result<HashSet<u64>> {
    let addresses = value
        .as_array()
        .ok_or("REA returned a malformed Rust durability address list")?;
    let values = addresses
        .iter()
        .map(|address| {
            address
                .as_str()
                .filter(|address| !address.is_empty())
                .and_then(crate::analysis::graph::address_value)
                .ok_or_else(|| failure("REA returned an invalid Rust durability address"))
        })
        .collect::<Result<Vec<_>>>()?;
    let unique = values.iter().copied().collect::<HashSet<_>>();
    if unique.len() != values.len() {
        return Err(failure("REA returned duplicate Rust durability addresses"));
    }
    Ok(unique)
}

fn validate_coverage(
    coverage: &Value,
    batch: &DecompilationBatch,
    end: usize,
    count: usize,
) -> Result<()> {
    if number(&coverage["examined"])? != count {
        return Err(failure("REA returned inconsistent page coverage"));
    }
    let final_page = end == batch.addresses.len();
    if coverage["exhausted"].as_bool() != Some(final_page)
        || (final_page && !coverage["next_offset"].is_null())
        || (!final_page && number(&coverage["next_offset"])? != end)
    {
        return Err(failure(
            "REA returned a decompilation page that did not advance correctly",
        ));
    }
    Ok(())
}

fn number(value: &Value) -> Result<usize> {
    usize::try_from(
        value
            .as_u64()
            .ok_or("REA omitted numeric decompilation coverage")?,
    )
    .map_err(Into::into)
}
