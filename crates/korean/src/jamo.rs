use crate::hangul::한글;
use std::fmt;

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
pub enum 자모 {
    초성(u16),
    중성(u16),
    종성(u16),
}

impl 자모 {
    pub fn unwrap_u16(&self) -> u16 {
        match self {
            자모::초성(c)
            | 자모::중성(c)
            | 자모::종성(c) => *c,
        }
    }

    pub fn into_char(&self) -> char {
        char::from_u32(self.unwrap_u16() as u32).unwrap()
    }
}

// Assumption: `s` is purely in 한글.
pub fn into_자모s(s: Vec<char>) -> Vec<자모> {
    let mut result = Vec::with_capacity(s.len() * 3);

    for c in s.into_iter() {
        let 한글 { 초성, 중성, 종성 } = 한글::from_char(c);
        result.push(자모::초성(초성));
        result.push(자모::중성(중성));

        if let Some(종성) = 종성 {
            result.push(자모::종성(종성));
        }
    }

    result
}

enum AssembleState {
    // expecting 초성
    초성,

    // expecting 중성
    중성(u16),

    // expecting 종성 or 초성
    종성(u16, u16),
}

// It assumes that `js` always contains a valid 한글 string.
pub fn assemble(js: &[자모]) -> String {
    let mut curr_state = AssembleState::초성;
    let mut result = vec![];

    for j in js.iter() {
        match curr_state {
            AssembleState::초성 => {
                let 자모::초성(c) = j else { unreachable!() };
                curr_state = AssembleState::중성(*c);
            },
            AssembleState::중성(c1) => {
                let 자모::중성(c2) = j else { unreachable!() };
                curr_state = AssembleState::종성(c1, *c2);
            },
            AssembleState::종성(c1, c2) => match j {
                자모::초성(c3) => {
                    result.push(한글 {
                        초성: c1,
                        중성: c2,
                        종성: None,
                    }.to_char());
                    curr_state = AssembleState::중성(*c3);
                },
                자모::중성(_) => unreachable!(),
                자모::종성(c3) => {
                    result.push(한글 {
                        초성: c1,
                        중성: c2,
                        종성: Some(*c3),
                    }.to_char());
                    curr_state = AssembleState::초성;
                },
            },
        }
    }

    if let AssembleState::종성(c1, c2) = curr_state {
        result.push(한글 {
            초성: c1,
            중성: c2,
            종성: None,
        }.to_char());
    }

    result.into_iter().collect()
}
