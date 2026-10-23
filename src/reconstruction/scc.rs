use std::collections::{BTreeMap, BTreeSet};

pub(super) fn components(graph: &BTreeMap<String, Vec<String>>) -> Vec<Vec<String>> {
    let mut seen = BTreeSet::new();
    let mut order = Vec::with_capacity(graph.len());
    for node in graph.keys() {
        finish_order(node, graph, &mut seen, &mut order);
    }
    let reversed = reverse(graph);
    seen.clear();
    let mut result = Vec::new();
    for node in order.into_iter().rev() {
        if seen.contains(&node) {
            continue;
        }
        let mut component = Vec::new();
        collect_iterative(&node, &reversed, &mut seen, &mut component);
        component.sort();
        result.push(component);
    }
    result
}

fn finish_order(
    root: &str,
    graph: &BTreeMap<String, Vec<String>>,
    seen: &mut BTreeSet<String>,
    order: &mut Vec<String>,
) {
    if !seen.insert(root.to_owned()) {
        return;
    }
    let mut stack = vec![(root.to_owned(), false)];
    while let Some((node, expanded)) = stack.pop() {
        if expanded {
            order.push(node);
            continue;
        }
        stack.push((node.clone(), true));
        if let Some(children) = graph.get(&node) {
            for child in children.iter().rev() {
                if seen.insert(child.clone()) {
                    stack.push((child.clone(), false));
                }
            }
        }
    }
}

fn collect_iterative(
    root: &str,
    graph: &BTreeMap<String, Vec<String>>,
    seen: &mut BTreeSet<String>,
    component: &mut Vec<String>,
) {
    let mut stack = vec![root.to_owned()];
    seen.insert(root.to_owned());
    while let Some(node) = stack.pop() {
        component.push(node.clone());
        if let Some(children) = graph.get(&node) {
            for child in children {
                if seen.insert(child.clone()) {
                    stack.push(child.clone());
                }
            }
        }
    }
}

fn reverse(graph: &BTreeMap<String, Vec<String>>) -> BTreeMap<String, Vec<String>> {
    let mut result = graph
        .keys()
        .map(|node| (node.clone(), Vec::new()))
        .collect::<BTreeMap<_, _>>();
    for (node, children) in graph {
        for child in children {
            result
                .get_mut(child)
                .expect("validated graph node")
                .push(node.clone());
        }
    }
    result
}
