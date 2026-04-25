use alloc::vec;
use alloc::collections::BTreeMap;
use alloc::collections::BTreeSet;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;

use crate::constraint::Constraint;

#[derive(Debug, Clone)]
pub struct ConstrainedNode {
    pub name: String,
    pub constraints: Vec<Constraint>,
    pub priority: u8,
}

#[derive(Debug, Clone, Copy)]
pub enum CycleAction {
    Break,
    Fail,
}

#[derive(Debug)]
pub struct SolveResult {
    pub order: Vec<String>,
    pub cycles: Vec<(String, String)>,
}

pub fn topological_sort(
    nodes: &[ConstrainedNode],
    cycle_action: CycleAction,
) -> SolveResult {
    let name_set: BTreeSet<&str> = nodes.iter().map(|n| n.name.as_str()).collect();

    let mut deps: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for node in nodes {
        let mut node_deps = Vec::new();
        for c in &node.constraints {
            for r in c.references() {
                if name_set.contains(r) && r != node.name.as_str() {
                    node_deps.push(r);
                }
            }
        }
        deps.insert(node.name.as_str(), node_deps);
    }

    // Kahn's algorithm
    let mut in_degree: BTreeMap<&str, usize> = BTreeMap::new();
    let mut reverse_deps: BTreeMap<&str, Vec<&str>> = BTreeMap::new();

    for node in nodes {
        in_degree.entry(node.name.as_str()).or_insert(0);
    }

    for (node, node_deps) in &deps {
        for d in node_deps {
            reverse_deps.entry(*d).or_default().push(node);
            *in_degree.entry(node).or_insert(0) += 1;
        }
    }

    // Build name → priority lookup
    let priority: BTreeMap<&str, u8> = nodes.iter()
        .map(|n| (n.name.as_str(), n.priority))
        .collect();

    // Ready queue sorted by priority descending (highest priority first)
    let mut ready: Vec<&str> = Vec::new();
    for (&node, &deg) in &in_degree {
        if deg == 0 {
            ready.push(node);
        }
    }
    ready.sort_by(|a, b| {
        let pa = priority.get(a).copied().unwrap_or(0);
        let pb = priority.get(b).copied().unwrap_or(0);
        pb.cmp(&pa).then_with(|| a.cmp(b))
    });

    let mut order: Vec<String> = Vec::new();
    let mut visited: BTreeSet<&str> = BTreeSet::new();

    while let Some(node) = ready.first().copied() {
        ready.remove(0);
        order.push(String::from(node));
        visited.insert(node);

        if let Some(dependents) = reverse_deps.get(node) {
            for dep in dependents {
                if let Some(deg) = in_degree.get_mut(dep) {
                    *deg -= 1;
                    if *deg == 0 {
                        // Insert in priority order
                        let p = priority.get(dep).copied().unwrap_or(0);
                        let pos = ready.iter().position(|r| {
                            let rp = priority.get(r).copied().unwrap_or(0);
                            p > rp || (p == rp && *dep < *r)
                        }).unwrap_or(ready.len());
                        ready.insert(pos, dep);
                    }
                }
            }
        }
    }

    let mut cycles = Vec::new();

    // Detect nodes not in order (part of cycles)
    for node in nodes {
        if !visited.contains(node.name.as_str()) {
            // Find one cycle edge for reporting
            if let Some(node_deps) = deps.get(node.name.as_str()) {
                for d in node_deps {
                    if !visited.contains(d) {
                        cycles.push((String::from(node.name.as_str()), String::from(*d)));
                    }
                }
            }

            match cycle_action {
                CycleAction::Break => {
                    order.push(String::from(node.name.as_str()));
                }
                CycleAction::Fail => {}
            }
        }
    }

    SolveResult { order, cycles }
}

pub fn solve_schedule(nodes: &[ConstrainedNode]) -> SolveResult {
    topological_sort(nodes, CycleAction::Break)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::String;

    fn node(name: &str, constraints: Vec<Constraint>) -> ConstrainedNode {
        ConstrainedNode {
            name: String::from(name),
            constraints,
            priority: 0,
        }
    }

    #[test]
    fn test_linear_chain() {
        let nodes = vec![
            node("c", vec![Constraint::After(String::from("b"))]),
            node("a", vec![]),
            node("b", vec![Constraint::After(String::from("a"))]),
        ];
        let result = solve_schedule(&nodes);
        let a_pos = result.order.iter().position(|n| n == "a").unwrap();
        let b_pos = result.order.iter().position(|n| n == "b").unwrap();
        let c_pos = result.order.iter().position(|n| n == "c").unwrap();
        assert!(a_pos < b_pos);
        assert!(b_pos < c_pos);
        assert!(result.cycles.is_empty());
    }

    #[test]
    fn test_no_constraints() {
        let nodes = vec![
            node("x", vec![]),
            node("y", vec![]),
            node("z", vec![]),
        ];
        let result = solve_schedule(&nodes);
        assert_eq!(result.order.len(), 3);
        assert!(result.cycles.is_empty());
    }

    #[test]
    fn test_cycle_detected() {
        let nodes = vec![
            node("a", vec![Constraint::After(String::from("b"))]),
            node("b", vec![Constraint::After(String::from("a"))]),
        ];
        let result = topological_sort(&nodes, CycleAction::Fail);
        assert!(!result.cycles.is_empty());
        assert!(result.order.is_empty());
    }

    #[test]
    fn test_cycle_broken() {
        let nodes = vec![
            node("a", vec![Constraint::After(String::from("b"))]),
            node("b", vec![Constraint::After(String::from("a"))]),
        ];
        let result = topological_sort(&nodes, CycleAction::Break);
        assert_eq!(result.order.len(), 2);
        assert!(!result.cycles.is_empty());
    }

    #[test]
    fn test_diamond_dependency() {
        let nodes = vec![
            node("d", vec![
                Constraint::After(String::from("b")),
                Constraint::After(String::from("c")),
            ]),
            node("b", vec![Constraint::After(String::from("a"))]),
            node("c", vec![Constraint::After(String::from("a"))]),
            node("a", vec![]),
        ];
        let result = solve_schedule(&nodes);
        let a_pos = result.order.iter().position(|n| n == "a").unwrap();
        let b_pos = result.order.iter().position(|n| n == "b").unwrap();
        let c_pos = result.order.iter().position(|n| n == "c").unwrap();
        let d_pos = result.order.iter().position(|n| n == "d").unwrap();
        assert!(a_pos < b_pos);
        assert!(a_pos < c_pos);
        assert!(b_pos < d_pos);
        assert!(c_pos < d_pos);
        assert!(result.cycles.is_empty());
    }

    #[test]
    fn test_shared_memory_creates_dependency() {
        let nodes = vec![
            node("writer", vec![]),
            node("reader", vec![Constraint::ShareMemory(String::from("writer"))]),
        ];
        let result = solve_schedule(&nodes);
        let w = result.order.iter().position(|n| n == "writer").unwrap();
        let r = result.order.iter().position(|n| n == "reader").unwrap();
        assert!(w < r);
    }

    #[test]
    fn test_priority_ordering() {
        let nodes = vec![
            ConstrainedNode {
                name: String::from("alogd"),
                constraints: vec![],
                priority: 1,
            },
            ConstrainedNode {
                name: String::from("zirq"),
                constraints: vec![],
                priority: 10,
            },
        ];
        let result = solve_schedule(&nodes);
        let a = result.order.iter().position(|n| n == "alogd").unwrap();
        let z = result.order.iter().position(|n| n == "zirq").unwrap();
        assert!(z < a, "higher priority 'zirq' should come before 'alogd'");
    }
}
