use bq_viewer::error::ParseError;
use bq_viewer::importance::*;
use bq_viewer::model::*;
use std::collections::HashMap;

fn qid(h: i32, l: i32) -> QuestId {
    QuestId { high: h, low: l }
}

fn make_db(quests: Vec<(QuestId, Vec<QuestId>)>) -> QuestDatabase {
    let mut map = HashMap::new();
    for (id, prereqs) in quests {
        let q = Quest {
            id: id.clone(),
            properties: None,
            tasks: vec![],
            rewards: vec![],
            prerequisites: prereqs.clone(),
            required_prerequisites: prereqs,
            optional_prerequisites: vec![],
        };
        map.insert(id, q);
    }
    QuestDatabase {
        settings: None,
        quests: map,
        questlines: HashMap::new(),
        questline_order: vec![],
    }
}

#[test]
fn star_topology() {
    // center C is prereq for A,B,D
    let c = qid(0, 0);
    let a = qid(0, 1);
    let b = qid(0, 2);
    let d = qid(0, 3);
    let db = make_db(vec![
        (c.clone(), vec![]),
        (a.clone(), vec![c.clone()]),
        (b.clone(), vec![c.clone()]),
        (d.clone(), vec![c.clone()]),
    ]);
    let scores = compute_importance_scores(&db, 0.25, false, true).unwrap();
    // center should have highest score
    let max = scores
        .iter()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
        .unwrap();
    assert_eq!(max.0.as_u64(), c.as_u64());
}

#[test]
fn chain_propagation() {
    // A -> B -> C (A is prereq for B; B prereq for C)
    let a = qid(0, 1);
    let b = qid(0, 2);
    let c = qid(0, 3);
    let db = make_db(vec![
        (a.clone(), vec![]),
        (b.clone(), vec![a.clone()]),
        (c.clone(), vec![b.clone()]),
    ]);
    let scores = compute_importance_scores(&db, 0.5, false, false).unwrap();
    // base counts: A has 1 dependent (B), B has 1 (C), C has 0
    // with alpha=0.5 and no log: score(A)=1 + 0.5*base(B)=1+0.5*1=1.5
    // score(B)=1 + 0.5*0 =1; score(C)=0
    assert!((scores.get(&a).cloned().unwrap() - 1.5).abs() < 1e-9);
    assert!((scores.get(&b).cloned().unwrap() - 1.0).abs() < 1e-9);
    assert!((scores.get(&c).cloned().unwrap() - 0.0).abs() < 1e-9);
}

#[test]
fn detect_cycle() {
    // A -> B -> C -> A
    let a = qid(0, 1);
    let b = qid(0, 2);
    let c = qid(0, 3);
    let db = make_db(vec![
        (a.clone(), vec![c.clone()]),
        (b.clone(), vec![a.clone()]),
        (c.clone(), vec![b.clone()]),
    ]);
    let res = compute_importance_scores(&db, 0.25, false, true);
    match res {
        Err(ParseError::CycleDetected(cycle)) => {
            // cycle should include at least one of the test ids
            assert!(cycle.iter().any(|q| q.as_u64() == a.as_u64()
                || q.as_u64() == b.as_u64()
                || q.as_u64() == c.as_u64()));
        }
        _ => panic!("expected cycle error"),
    }
}
