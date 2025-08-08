use crate::MoccaId;
use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Default)]
pub struct DepTree {
    // Edge: u -> v means v depends on u (u must run before v)
    succ: HashMap<MoccaId, HashSet<MoccaId>>,
    nodes: HashMap<MoccaId, String>,
}

impl DepTree {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn contains_node(&self, id: MoccaId) -> bool {
        self.nodes.contains_key(&id)
    }

    pub fn add_node(&mut self, a: MoccaId, name: String) {
        self.nodes.insert(a, name);
    }

    /// Declare a dependency: `a` depends on `b`.
    pub fn add_dep(&mut self, a: MoccaId, b: MoccaId) {
        assert!(self.contains_node(a));
        assert!(self.contains_node(b));
        self.succ.entry(b).or_default().insert(a); // b -> a
    }

    /// Order where each item executes after its dependencies (deps come first).
    pub fn order_after_deps(&self) -> Result<Vec<MoccaId>, String> {
        // indegree[v] = number of prerequisites (incoming edges) for v
        let mut indegree: HashMap<MoccaId, usize> =
            self.nodes.keys().copied().map(|n| (n, 0)).collect();

        for (_u, vs) in &self.succ {
            for &v in vs {
                *indegree.get_mut(&v).unwrap() += 1;
            }
        }

        let mut q: VecDeque<MoccaId> = indegree
            .iter()
            .filter(|(_, d)| **d == 0)
            .map(|(&n, _)| n)
            .collect();

        let mut out = Vec::with_capacity(self.nodes.len());

        while let Some(u) = q.pop_front() {
            out.push(u);
            if let Some(vs) = self.succ.get(&u) {
                for &v in vs {
                    let d = indegree.get_mut(&v).unwrap();
                    *d -= 1;
                    if *d == 0 {
                        q.push_back(v);
                    }
                }
            }
        }

        if out.len() == self.nodes.len() {
            Ok(out)
        } else {
            Err("dependency cycle detected".into())
        }
    }

    /// Order where each item executes before its dependencies (dependents come first).
    pub fn order_before_deps(&self) -> Result<Vec<MoccaId>, String> {
        let mut v = self.order_after_deps()?;
        v.reverse();
        Ok(v)
    }
}

impl DepTree {
    // NOTE: ChatGPT
    pub fn print_tree(&self) {
        // Build reverse edges: preds[n] = direct dependencies of n
        let mut preds: HashMap<MoccaId, HashSet<MoccaId>> = HashMap::new();
        for (&dep, dependents) in &self.succ {
            for &d in dependents {
                preds.entry(d).or_default().insert(dep);
            }
        }
        for id in self.nodes.keys() {
            preds.entry(*id).or_default();
        }

        // Longest dependency depth (memoized)
        let mut memo: HashMap<MoccaId, usize> = HashMap::new();
        fn depth_of(
            n: MoccaId,
            preds: &HashMap<MoccaId, HashSet<MoccaId>>,
            memo: &mut HashMap<MoccaId, usize>,
            stack: &mut HashSet<MoccaId>,
        ) -> usize {
            if let Some(&d) = memo.get(&n) {
                return d;
            }
            if !stack.insert(n) {
                memo.insert(n, 0);
                return 0;
            }
            let d = preds
                .get(&n)
                .map(|ps| {
                    ps.iter()
                        .map(|&p| depth_of(p, preds, memo, stack) + 1)
                        .max()
                        .unwrap_or(0)
                })
                .unwrap_or(0);
            stack.remove(&n);
            memo.insert(n, d);
            d
        }
        for &id in self.nodes.keys() {
            depth_of(id, &preds, &mut memo, &mut HashSet::new());
        }

        // Start nodes sorted by depth desc, then by name
        let mut starts: Vec<_> = self.nodes.keys().copied().collect();
        starts.sort_by(|a, b| {
            memo[b]
                .cmp(&memo[a])
                .then_with(|| self.nodes[a].cmp(&self.nodes[b]))
        });

        let mut printed: HashSet<MoccaId> = HashSet::new();
        for id in starts {
            if printed.contains(&id) {
                continue;
            }
            // Root line
            println!("{}", self.nodes[&id]);
            printed.insert(id);
            // Print descendants
            self.print_children(id, "", &preds, &mut HashSet::new(), &mut printed);
            println!();
        }
    }

    // NOTE: ChatGPT
    /// Print only the descendants of `id` (not the node itself).
    fn print_children(
        &self,
        id: MoccaId,
        prefix: &str,
        preds: &HashMap<MoccaId, HashSet<MoccaId>>,
        visiting: &mut HashSet<MoccaId>,
        printed: &mut HashSet<MoccaId>,
    ) {
        if !visiting.insert(id) {
            println!("{}└── (cycle)", prefix);
            return;
        }

        // Dependencies of `id` (children in the printed tree)
        let mut deps: Vec<MoccaId> = preds
            .get(&id)
            .map(|s| s.iter().copied().collect())
            .unwrap_or_else(Vec::new);
        deps.sort_by(|a, b| self.nodes[a].cmp(&self.nodes[b]));

        for (i, child) in deps.clone().into_iter().enumerate() {
            let is_last = i + 1 == deps.len();
            let branch = if is_last { "└── " } else { "├── " };
            let next_prefix = if is_last {
                format!("{}    ", prefix)
            } else {
                format!("{}│   ", prefix)
            };

            // Print the child node exactly once here
            println!("{}{}{}", prefix, branch, self.nodes[&child]);
            printed.insert(child);

            // Recurse to print the child's descendants (not the child itself again)
            self.print_children(child, &next_prefix, preds, visiting, printed);
        }

        visiting.remove(&id);
    }
}
