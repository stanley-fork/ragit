use crate::hangul::{종성s, 중성s, 한글};
use crate::jamo::자모;
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum TermKind {
    종성Term,
    Non종성Term,
    Any,

    // represents a TermKind that's lowered from `종성Term` or `Non종성Term`, by `flatten_term_kinds`.
    Offset,
}

// cargo run --release > fsm.rs; mv fsm.rs src/fsm.rs; cargo test --release -- --nocapture
pub fn gen_fsm(debug: bool) {
    let rules = vec![
        (TermKind::종성Term, vec![vec!["은", "이", "을", "과", "으로", "이랑", "이라고"]]),
        (TermKind::Non종성Term, vec![vec!["는", "가", "를", "와", "랑", "라고"]]),
        (TermKind::Any, vec![vec!["의", "만", "도", "에", "에서", "로", "까지", "부터", "한테", "하고", "께"]]),
        (TermKind::Any, vec![vec!["이", "하"], vec!["ㅂ니다", "ㄴ데", "ㄴ지", "고", "면", "다", "지만"]]),
    ];

    let rules = disassemble_자모s(rules);
    let rules = flatten_term_kinds(rules);
    let rules = flatten_rules1(rules);
    let rules = flatten_rules2(rules);
    let states = build_states(rules);
    let code = generate_code(states, debug);

    println!("{code}");
}

fn disassemble_자모s(rules: Vec<(TermKind, Vec<Vec<&str>>)>) -> Vec<(TermKind, Vec<Vec<Vec<자모>>>)> {
    rules.into_iter().map(
        |(term, suffix)| (
            term,
            suffix.into_iter().map(
                |ss| ss.into_iter().map(
                    |s| {
                        let mut result = Vec::with_capacity(s.len() * 3);

                        for c in s.chars() {
                            if 'ㄱ' <= c && c <= 'ㅎ' {
                                result.push(자모::종성(c as u16));
                            }

                            else {
                                let 한글 { 초성, 중성, 종성 } = 한글::from_char(c);
                                result.push(자모::초성(초성));
                                result.push(자모::중성(중성));

                                if let Some(종성) = 종성 {
                                    result.push(자모::종성(종성));
                                }
                            }
                        }

                        result
                    }
                ).collect()
            ).collect()
        )
    ).collect()
}

// (종성Term, [[은, 이, 을, 과]]) -> (Any, [[ㄱ, ㄴ, ㄷ, ...], [은, 이, 을, 과]])
fn flatten_term_kinds(mut rules: Vec<(TermKind, Vec<Vec<Vec<자모>>>)>) -> Vec<(TermKind, Vec<Vec<Vec<자모>>>)> {
    let 중성_자모s = 중성s.iter().map(|j| vec![자모::중성(*j)]).collect::<Vec<_>>();
    let 종성_자모s = 종성s.iter().map(|j| vec![자모::종성(*j)]).collect::<Vec<_>>();

    for (term, suffixes) in rules.iter_mut() {
        match term {
            TermKind::종성Term => {
                suffixes.insert(0, 종성_자모s.clone());
                *term = TermKind::Offset;
            },
            TermKind::Non종성Term => {
                suffixes.insert(0, 중성_자모s.clone());
                *term = TermKind::Offset;
            },
            TermKind::Any => {},
            TermKind::Offset => unreachable!(),
        }
    }

    rules
}

// [[이, 하], [ㅂ니다, ㄴ데, ㄴ지, 고]] -> [입니다, 인데, 인지, 이고, 합니다, 한데, 한지, 하고]
fn flatten_rules1(rules: Vec<(TermKind, Vec<Vec<Vec<자모>>>)>) -> Vec<(TermKind, Vec<Vec<자모>>)> {
    let mut result = Vec::with_capacity(rules.len());

    for (term, mut suffix) in rules.into_iter() {
        loop {
            if suffix.len() == 1 {
                result.push((term, suffix[0].clone()));
                break;
            }

            else {
                let l2 = suffix.pop().unwrap();
                let l1 = suffix.pop().unwrap();
                let mut permutation = Vec::with_capacity(l1.len() * l2.len());

                for j1 in l1.iter() {
                    for j2 in l2.iter() {
                        permutation.push(vec![
                            j1.clone(),
                            j2.clone(),
                        ].concat());
                    }
                }

                suffix.push(permutation);
            }
        }
    }

    result
}

fn flatten_rules2(rules: Vec<(TermKind, Vec<Vec<자모>>)>) -> Vec<(TermKind, Vec<자모>)> {
    let mut result = vec![];

    for (term, suffixes) in rules.into_iter() {
        for suffix in suffixes.into_iter() {
            result.push((term, suffix));
        }
    }

    result
}

type StateId = usize;

#[derive(Debug)]
struct State {
    remaining_units: Vec<(TermKind, Vec<자모>)>,
    transitions: HashMap<자모, StateId>,
    terminations: Vec<TermKind>,
}

fn build_states(units: Vec<(TermKind, Vec<자모>)>) -> Vec<State> {
    let mut result = vec![];
    result.push(
        State {
            remaining_units: units,
            transitions: HashMap::new(),
            terminations: vec![],
        },
    );

    loop {
        let mut transitions = vec![];

        for (state_id, state) in result.iter_mut().enumerate() {
            let mut terminations = vec![];

            for (term, 자모s) in state.remaining_units.iter() {
                if !자모s.is_empty() {
                    let mut 자모s = 자모s.clone();
                    transitions.push((state_id, 자모s.pop().unwrap(), (*term, 자모s)));
                }

                else {
                    terminations.push(*term);
                }
            }

            for termination in terminations.into_iter() {
                if !state.terminations.contains(&termination) {
                    state.terminations.push(termination);
                }
            }

            state.remaining_units = vec![];
        }

        if transitions.is_empty() {
            break;
        }

        for (prev_state, 자모, remaining_unit) in transitions.into_iter() {
            let result_len = result.len();
            let prev_state = result.get_mut(prev_state).unwrap();

            let new_state_id = match prev_state.transitions.get(&자모) {
                Some(new_state_id) => *new_state_id,
                None => {
                    prev_state.transitions.insert(자모, result_len);
                    result_len
                },
            };

            match result.get_mut(new_state_id) {
                Some(state) => {
                    state.remaining_units.push(remaining_unit);
                },
                None => {
                    result.push(
                        State {
                            remaining_units: vec![remaining_unit],
                            transitions: HashMap::new(),
                            terminations: vec![],
                        },
                    );
                },
            }
        }
    }

    result
}

fn generate_code(states: Vec<State>, debug: bool) -> String {
    // (indent, content)
    let mut lines = vec![];

    lines.push((0, String::from("use crate::jamo::{자모, assemble};")));
    lines.push((0, String::new()));
    lines.push((0, String::from("// This code is auto-generated by gen.rs")));
    lines.push((0, String::from("pub fn fsm(자모s: Vec<자모>) -> String {")));
    lines.push((1, String::from("let mut curr_state = 0;")));
    lines.push((1, String::from("let mut count = 0;")));
    lines.push((1, String::new()));
    lines.push((1, String::from("let cut_at = loop {")));
    lines.push((2, String::from("count += 1;")));
    lines.push((2, String::from("let curr_char = if count <= 자모s.len() { 자모s.get(자모s.len() - count) } else { None };")));

    if debug {
        lines.push((2, String::from("println!(\"curr_state: {curr_state}\");")));
        lines.push((2, String::from("println!(\"curr_char: {curr_char:?}, {:?}\", curr_char.map(|c| c.into_char()));")));
    }

    lines.push((2, String::new()));
    lines.push((2, String::from("match curr_state {")));

    for (id, state) in states.iter().enumerate() {
        assert!(state.remaining_units.is_empty());
        lines.push((3, format!("{id} => match curr_char {}", '{')));

        // the generated code must be deterministic
        let mut transitions = state.transitions.iter().map(|(자모, state_id)| (*자모, *state_id)).collect::<Vec<_>>();
        transitions.sort_by_key(|(자모, _)| 자모.into_char());

        for (자모, state_id) in transitions.iter() {
            lines.push((4, format!(
                "Some(자모::{자모:?}  /* {} */) => {} curr_state = {state_id}; continue; {},",
                char::from_u32(자모.unwrap_u16() as u32).unwrap(),
                "{", "}",
            )));
        }

        if state.terminations.is_empty() {
            lines.push((4, format!("_ => {} break 0; {},", "{", "}")));
        }

        else {
            assert_eq!(state.terminations.len(), 1);

            match state.terminations[0] {
                TermKind::Any => {
                    lines.push((4, format!("_ => {} break count - 1; {},", "{", "}")));
                },
                TermKind::Offset => {
                    lines.push((4, format!("_ => {} break count - 2; {},", "{", "}")));
                },
                _ => unreachable!(),
            }
        }

        lines.push((3, format!("{},", '}')));
    }

    lines.push((3, String::from("_ => unreachable!(),")));
    lines.push((2, String::from("}")));
    lines.push((1, String::from("};")));
    lines.push((1, String::new()));

    if debug {
        lines.push((1, String::from("println!(\"cut_at: {cut_at}\");")));
        lines.push((1, String::from("println!(\"자모s (all): {:?}\", 자모s.iter().map(|j| j.into_char()).collect::<Vec<_>>());")));
    }

    lines.push((1, String::from("assemble(&자모s[..(자모s.len() - cut_at)].to_vec())")));
    lines.push((0, String::from("}")));

    let lines = lines.into_iter().map(
        |(indent, content)| if content.is_empty() {
            content
        } else {
            format!("{}{content}", " ".repeat(indent * 4))
        }
    ).collect::<Vec<_>>();
    lines.join("\n")
}
