use super::{
    Access, Base, Convention, Evidence, Field, Kind, Model, RecoveredPrototype, RecoveredType,
    RecoveredValue, ValueType,
};
use crate::error::Result;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

type TypeIds = BTreeMap<Base, String>;

#[cfg_attr(feature = "profiling", hotpath::measure)]
pub(crate) fn infer(evidence: &Evidence) -> Result<Model> {
    evidence.validate()?;
    let groups = pointer_groups(evidence);
    let ids: TypeIds = groups
        .iter()
        .flat_map(|(id, bases)| bases.iter().map(|base| (base.clone(), id.clone())))
        .collect();
    let mut model = Model {
        types: Vec::new(),
        prototypes: Vec::new(),
        conflicts: Vec::new(),
    };
    for (id, consumers) in groups {
        let recovered = recover_layout(evidence, &ids, id, consumers, &mut model.conflicts)?;
        model.types.push(recovered);
    }
    check_names(&model.types, &mut model.conflicts);
    model.prototypes = recover_prototypes(evidence, &ids, &mut model.conflicts);
    Ok(model)
}

fn pointer_groups(evidence: &Evidence) -> BTreeMap<String, Vec<Base>> {
    let mut remaining: BTreeSet<Base> = evidence.bases().into_iter().cloned().collect();
    let mut groups = BTreeMap::new();
    while let Some(first) = remaining.pop_first() {
        let digest = Sha256::digest(format!("{}:{}", first.function, first.variable));
        let id = format!("type_{digest:x}");
        let mut pending = vec![first];
        let mut members = BTreeSet::new();
        while let Some(base) = pending.pop() {
            for alias in &evidence.aliases {
                let neighbor = if alias.left == base {
                    Some(&alias.right)
                } else if alias.right == base {
                    Some(&alias.left)
                } else {
                    None
                };
                if let Some(neighbor) = neighbor
                    && remaining.remove(neighbor)
                {
                    pending.push(neighbor.clone());
                }
            }
            members.insert(base);
        }
        groups.insert(id, members.into_iter().collect());
    }
    groups
}

fn recover_layout(
    evidence: &Evidence,
    ids: &TypeIds,
    id: String,
    consumers: Vec<Base>,
    conflicts: &mut Vec<String>,
) -> Result<RecoveredType> {
    let name = recovered_name(evidence, &consumers, &id, conflicts);
    let fields = recover_fields(evidence, ids, &consumers, &name, conflicts)?;
    let minimum_size = fields.iter().map(field_end).max().unwrap_or_default();
    let shape = layout_shape(&fields, evidence.pointer_size);
    Ok(RecoveredType {
        id,
        name,
        shape,
        minimum_size,
        consumers,
        fields,
    })
}

fn recovered_name(
    evidence: &Evidence,
    consumers: &[Base],
    id: &str,
    conflicts: &mut Vec<String>,
) -> String {
    let names: BTreeSet<&str> = evidence
        .names
        .iter()
        .filter(|name| consumers.contains(&name.base))
        .map(|name| name.name.as_str())
        .collect();
    if names.len() > 1 {
        conflict(
            conflicts,
            format!("{id}: aliases have different proposed type names: {names:?}"),
        );
    }
    names
        .first()
        .map_or_else(|| format!("Recovered_{id}"), |name| (*name).to_owned())
}

fn recover_fields(
    evidence: &Evidence,
    ids: &TypeIds,
    consumers: &[Base],
    name: &str,
    conflicts: &mut Vec<String>,
) -> Result<Vec<Field>> {
    let mut accesses: Vec<&Access> = evidence
        .accesses
        .iter()
        .filter(|access| consumers.contains(&access.base))
        .collect();
    // Larger array observations precede their individual element observations.
    accesses.sort_by_key(|access| (access.offset, std::cmp::Reverse(access.end())));

    let mut fields: Vec<Field> = Vec::new();
    for access in accesses {
        let end = access.end().ok_or("The access extent overflowed")?;
        let pointee = access
            .pointee
            .as_ref()
            .and_then(|base| ids.get(base))
            .cloned();
        if let Some(field) = fields
            .iter_mut()
            .find(|field| covers(field, access, pointee.as_deref()))
        {
            merge_access(field, access, pointee);
            continue;
        }
        for field in &fields {
            let field_end = field_end(field);
            if access.offset < field_end && field.offset < end {
                conflict(
                    conflicts,
                    format!(
                        "{name}: incompatible accesses at byte offsets {} and {} ({} in {})",
                        field.offset, access.offset, access.site, access.base.function
                    ),
                );
            }
        }
        fields.push(Field {
            offset: access.offset,
            width: access.width,
            kind: access.kind,
            count: access.count,
            stride: access.stride(),
            pointee,
            targets: access.target.iter().cloned().collect(),
            evidence: vec![access.clone()],
        });
    }
    Ok(fields)
}

fn covers(field: &Field, access: &Access, pointee: Option<&str>) -> bool {
    if field.kind != access.kind
        || field.width != access.width
        || (field.pointee.is_some() && pointee.is_some() && field.pointee.as_deref() != pointee)
    {
        return false;
    }
    let Some(delta) = access.offset.checked_sub(field.offset) else {
        return false;
    };
    delta.is_multiple_of(field.stride)
        && (access.count == 1 || access.stride() == field.stride)
        && access.end().is_some_and(|end| end <= field_end(field))
}

fn field_end(field: &Field) -> u64 {
    // Fields are constructed exclusively from validated, bounded access extents.
    field.offset + (field.count - 1) * field.stride + field.width
}

fn merge_access(field: &mut Field, access: &Access, pointee: Option<String>) {
    if field.pointee.is_none() {
        field.pointee = pointee;
    }
    if let Some(target) = &access.target
        && !field.targets.contains(target)
    {
        field.targets.push(target.clone());
        field.targets.sort();
    }
    if !field.evidence.contains(access) {
        field.evidence.push(access.clone());
    }
}

fn layout_shape(fields: &[Field], pointer_size: u64) -> &'static str {
    if fields.is_empty() {
        return "opaque";
    }
    let mut next = 0;
    let mut slots = 0;
    for field in fields {
        if field.kind != Kind::FunctionPointer
            || field.width != pointer_size
            || field.offset != next
            || field.stride != pointer_size
        {
            return "structure";
        }
        slots += field.count;
        next += field.count * pointer_size;
    }
    if slots >= 2 {
        "possible_vtable"
    } else {
        "structure"
    }
}

fn check_names(types: &[RecoveredType], conflicts: &mut Vec<String>) {
    let mut names = BTreeSet::new();
    for recovered in types {
        if !names.insert(&recovered.name) {
            conflict(
                conflicts,
                format!(
                    "{}: the same type name describes unrelated pointer groups",
                    recovered.name
                ),
            );
        }
    }
}

fn recover_prototypes(
    evidence: &Evidence,
    ids: &TypeIds,
    conflicts: &mut Vec<String>,
) -> Vec<RecoveredPrototype> {
    let mut functions: BTreeMap<&str, RecoveredPrototype> = BTreeMap::new();
    for observation in &evidence.prototypes {
        if !super::validation::convention_matches(observation.convention, evidence.pointer_size) {
            conflict(
                conflicts,
                format!(
                    "{}: calling convention {:?} contradicts pointer size",
                    observation.function, observation.convention
                ),
            );
        }
        let parameters: Vec<_> = observation
            .parameters
            .iter()
            .map(|value| recover_value(value, ids))
            .collect();
        let returns = observation
            .returns
            .as_ref()
            .map(|value| recover_value(value, ids));
        let Some(prototype) = functions.get_mut(observation.function.as_str()) else {
            functions.insert(
                &observation.function,
                RecoveredPrototype {
                    function: observation.function.clone(),
                    convention: observation.convention,
                    parameters,
                    returns,
                    evidence: vec![observation.clone()],
                },
            );
            continue;
        };
        let convention_conflict = prototype.convention != Convention::Unknown
            && observation.convention != Convention::Unknown
            && prototype.convention != observation.convention;
        if prototype.parameters != parameters || prototype.returns != returns || convention_conflict
        {
            conflict(
                conflicts,
                format!(
                    "{}: prototype observations disagree at {}",
                    observation.function, observation.site
                ),
            );
        }
        if prototype.convention == Convention::Unknown {
            prototype.convention = observation.convention;
        }
        if !prototype.evidence.contains(observation) {
            prototype.evidence.push(observation.clone());
        }
    }
    functions.into_values().collect()
}

fn recover_value(value: &ValueType, ids: &TypeIds) -> RecoveredValue {
    RecoveredValue {
        kind: value.kind,
        width: value.width,
        pointee: value
            .pointee
            .as_ref()
            .and_then(|base| ids.get(base))
            .cloned(),
    }
}

fn conflict(conflicts: &mut Vec<String>, message: String) {
    match conflicts.len() {
        0..128 => conflicts.push(message),
        128 => conflicts
            .push("Further conflicts omitted; resolve these observations and check again".into()),
        _ => {}
    }
}
