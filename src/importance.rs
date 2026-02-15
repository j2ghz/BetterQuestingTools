use crate::error::{ParseError, Result};
use crate::model::*;
use crate::quest_id::QuestId;
use std::collections::{HashMap, HashSet};

/// Compute one-step importance scores for quests in `db`.
///
/// - `alpha` is the propagation factor (0.0..1.0) applied to dependent bases.
/// - `use_log` applies ln(1 + raw_count) compression to base counts.
/// - `normalize` rescales final scores into [0, 1) (max strictly less than 1).
pub fn compute_importance_scores(
    db: &QuestDatabase,
    alpha: f64,
    use_log: bool,
    normalize: bool,
) -> Result<HashMap<QuestId, f64>> {
    if !(0.0..=1.0).contains(&alpha) {
        return Err(ParseError::AlphaOutOfRange(alpha));
    }

    // Build adjacency (quest -> its prerequisites) for cycle detection and
    // dependents map (prereq -> list of dependents with weights).
    let mut adj: HashMap<QuestId, Vec<QuestId>> = HashMap::new();
    let mut dependents: HashMap<QuestId, Vec<(QuestId, f64)>> = HashMap::new();

    for (qid, quest) in &db.quests {
        // Exclude all outgoing prerequisite edges for quests with quest_logic == "XOR"
        let is_xor = quest
            .properties
            .as_ref()
            .and_then(|props| props.quest_logic.as_deref())
            .is_some_and(|logic| logic.eq_ignore_ascii_case("XOR"));
        if is_xor {
            // Skip adding this quest's prerequisite edges to avoid cycles/weight propagation
            continue;
        }
        // dedupe prerequisites per quest to avoid double counting
        let mut seen: HashSet<u64> = HashSet::new();

        // prefer explicit required_prerequisites; otherwise fall back to
        // the generic `prerequisites` list. Optionals come from
        // `optional_prerequisites` when present.
        let base_required = if !quest.required_prerequisites.is_empty() {
            quest.required_prerequisites.clone()
        } else {
            quest.prerequisites.clone()
        };
        let base_optionals = quest.optional_prerequisites.clone();

        let mut required: Vec<QuestId> = Vec::new();
        for p in base_required.into_iter() {
            if seen.insert(p.as_u64()) {
                required.push(p);
            }
        }

        let mut optionals: Vec<QuestId> = Vec::new();
        for p in base_optionals.into_iter() {
            if seen.insert(p.as_u64()) {
                optionals.push(p);
            }
        }

        // adjacency should include both required and optional edges for cycle detection
        let mut adj_list = required.clone();
        adj_list.extend(optionals.iter().cloned());
        adj.insert(*qid, adj_list);

        // build dependents: required edges weight 1.0
        for p in required.iter().cloned() {
            dependents.entry(p).or_default().push((*qid, 1.0));
        }

        // optional edges split weight equally among the group's members
        if !optionals.is_empty() {
            let w = 1.0 / (optionals.len() as f64);
            for p in optionals.into_iter() {
                dependents.entry(p).or_default().push((*qid, w));
            }
        }
    }

    // Cycle detection on the adjacency graph (quest -> prerequisites). Any
    // directed cycle means the prerequisites graph is not a DAG and we fail.
    // We'll run DFS with 3-color marking and capture one cycle if present.
    enum Color {
        White,
        Gray,
        Black,
    }

    let mut color: HashMap<QuestId, Color> = HashMap::new();
    for k in db.quests.keys() {
        color.insert(*k, Color::White);
    }

    let mut stack: Vec<QuestId> = Vec::new();
    let mut pos_in_stack: HashMap<u64, usize> = HashMap::new();

    fn dfs_visit(
        node: &QuestId,
        adj: &HashMap<QuestId, Vec<QuestId>>,
        color: &mut HashMap<QuestId, Color>,
        stack: &mut Vec<QuestId>,
        pos_in_stack: &mut HashMap<u64, usize>,
    ) -> Option<Vec<QuestId>> {
        // mark gray
        color.insert(*node, Color::Gray);
        pos_in_stack.insert(node.as_u64(), stack.len());
        stack.push(*node);

        if let Some(neis) = adj.get(node) {
            for nei in neis {
                match color.get(nei) {
                    Some(Color::White) => {
                        if let Some(cycle) = dfs_visit(nei, adj, color, stack, pos_in_stack) {
                            return Some(cycle);
                        }
                    }
                    Some(Color::Gray) => {
                        // found a cycle: slice from pos_in_stack[nei]..end
                        if let Some(&start) = pos_in_stack.get(&nei.as_u64()) {
                            let cycle = stack[start..].to_vec();
                            return Some(cycle);
                        } else {
                            return Some(vec![*nei, *node]);
                        }
                    }
                    _ => {}
                }
            }
        }

        // mark black
        stack.pop();
        pos_in_stack.remove(&node.as_u64());
        color.insert(*node, Color::Black);
        None
    }

    for node in db.quests.keys() {
        if let Some(Color::White) = color.get(node)
            && let Some(cycle) = dfs_visit(node, &adj, &mut color, &mut stack, &mut pos_in_stack)
        {
            return Err(ParseError::CycleDetected(cycle));
        }
    }

    // Compute base scores: raw count of dependents (with weights). Keep exact
    // integer counts where possible (we represent as f64 for final math).
    let mut base: HashMap<QuestId, f64> = HashMap::new();
    for q in db.quests.keys() {
        let raw = dependents
            .get(q)
            .map(|v| v.iter().fold(0.0f64, |acc, (_dep, w)| acc + *w))
            .unwrap_or(0.0);
        let val = if use_log { (1.0 + raw).ln() } else { raw };
        base.insert(*q, val);
    }

    // Compute propagated one-step score: score = base + alpha * sum_{d in dependents} weight(d->q) * base(d)
    let mut score: HashMap<QuestId, f64> = HashMap::new();
    for q in db.quests.keys() {
        let b = *base.get(q).unwrap_or(&0.0);
        let prop = dependents
            .get(q)
            .map(|deps| {
                deps.iter().fold(0.0f64, |acc, (d, w)| {
                    acc + w * base.get(d).cloned().unwrap_or(0.0)
                })
            })
            .unwrap_or(0.0);
        score.insert(*q, b + alpha * prop);
    }

    // Normalize into [0,1) if requested. Ensure max maps strictly less than 1.
    if normalize {
        let max = score.values().cloned().fold(f64::NAN, f64::max);
        if max.is_nan() || max == 0.0 {
            // nothing to do
            return Ok(score);
        }
        let divisor = max * 1.000000001_f64; // tiny inflation guarantees < 1.0
        for v in score.values_mut() {
            *v /= divisor;
        }
    }

    Ok(score)
}

/// Order prerequisites for a given quest by importance using the precomputed
/// `scores` map. Returns a vector of (QuestId, score) sorted descending.
pub fn order_prereqs_for_quest(
    quest: &Quest,
    scores: &HashMap<QuestId, f64>,
) -> Vec<(QuestId, f64)> {
    let mut out: Vec<(QuestId, f64)> = quest
        .prerequisites
        .iter()
        .map(|q| (*q, *scores.get(q).unwrap_or(&0.0)))
        .collect();

    // deterministic sort: score desc, tie-break by QuestId.as_u64() asc
    out.sort_by(|(a_id, a_s), (b_id, b_s)| {
        match b_s.partial_cmp(a_s).unwrap_or(std::cmp::Ordering::Equal) {
            std::cmp::Ordering::Equal => a_id.as_u64().cmp(&b_id.as_u64()),
            ord => ord,
        }
    });
    out
}
