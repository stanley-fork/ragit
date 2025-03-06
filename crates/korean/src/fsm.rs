use crate::jamo::{자모, assemble};

// This code is auto-generated by generator.rs
pub fn fsm(자모s: Vec<자모>) -> String {
    let mut curr_state = 0;
    let mut count = 0;

    let cut_at = loop {
        count += 1;
        let curr_char = if count <= 자모s.len() { 자모s.get(자모s.len() - count) } else { None };

        match curr_state {
            0 => match curr_char {
                Some(자모::종성(12596)  /* ㄴ */) => { curr_state = 1; continue; },
                Some(자모::종성(12601)  /* ㄹ */) => { curr_state = 3; continue; },
                Some(자모::종성(12615)  /* ㅇ */) => { curr_state = 6; continue; },
                Some(자모::중성(12623)  /* ㅏ */) => { curr_state = 7; continue; },
                Some(자모::중성(12627)  /* ㅓ */) => { curr_state = 10; continue; },
                Some(자모::중성(12628)  /* ㅔ */) => { curr_state = 9; continue; },
                Some(자모::중성(12631)  /* ㅗ */) => { curr_state = 5; continue; },
                Some(자모::중성(12632)  /* ㅘ */) => { curr_state = 4; continue; },
                Some(자모::중성(12642)  /* ㅢ */) => { curr_state = 8; continue; },
                Some(자모::중성(12643)  /* ㅣ */) => { curr_state = 2; continue; },
                _ => { break 0; },
            },
            1 => match curr_char {
                Some(자모::중성(12623)  /* ㅏ */) => { curr_state = 12; continue; },
                Some(자모::중성(12629)  /* ㅕ */) => { curr_state = 13; continue; },
                Some(자모::중성(12641)  /* ㅡ */) => { curr_state = 11; continue; },
                _ => { break 0; },
            },
            2 => match curr_char {
                Some(자모::초성(12615)  /* ㅇ */) => { curr_state = 14; continue; },
                Some(자모::초성(12616)  /* ㅈ */) => { curr_state = 15; continue; },
                _ => { break 0; },
            },
            3 => match curr_char {
                Some(자모::중성(12641)  /* ㅡ */) => { curr_state = 16; continue; },
                _ => { break 0; },
            },
            4 => match curr_char {
                Some(자모::초성(12593)  /* ㄱ */) => { curr_state = 17; continue; },
                Some(자모::초성(12615)  /* ㅇ */) => { curr_state = 18; continue; },
                _ => { break 0; },
            },
            5 => match curr_char {
                Some(자모::초성(12593)  /* ㄱ */) => { curr_state = 20; continue; },
                Some(자모::초성(12599)  /* ㄷ */) => {  break count - 1;  },
                Some(자모::초성(12601)  /* ㄹ */) => { curr_state = 19; continue; },
                _ => { break 0; },
            },
            6 => match curr_char {
                Some(자모::중성(12623)  /* ㅏ */) => { curr_state = 22; continue; },
                _ => { break 0; },
            },
            7 => match curr_char {
                Some(자모::초성(12593)  /* ㄱ */) => { curr_state = 23; continue; },
                Some(자모::초성(12599)  /* ㄷ */) => { curr_state = 24; continue; },
                _ => { break 0; },
            },
            8 => match curr_char {
                Some(자모::초성(12615)  /* ㅇ */) => {  break count - 1;  },
                _ => { break 0; },
            },
            9 => match curr_char {
                Some(자모::초성(12593)  /* ㄱ */) => { curr_state = 27; continue; },
                Some(자모::초성(12594)  /* ㄲ */) => {  break count - 1;  },
                Some(자모::초성(12599)  /* ㄷ */) => { curr_state = 30; continue; },
                Some(자모::초성(12615)  /* ㅇ */) => {  break count - 1;  },
                Some(자모::초성(12620)  /* ㅌ */) => { curr_state = 28; continue; },
                _ => { break 0; },
            },
            10 => match curr_char {
                Some(자모::초성(12613)  /* ㅅ */) => { curr_state = 31; continue; },
                Some(자모::초성(12620)  /* ㅌ */) => { curr_state = 32; continue; },
                _ => { break 0; },
            },
            11 => match curr_char {
                Some(자모::초성(12596)  /* ㄴ */) => { curr_state = 34; continue; },
                Some(자모::초성(12615)  /* ㅇ */) => { curr_state = 33; continue; },
                _ => { break 0; },
            },
            12 => match curr_char {
                Some(자모::초성(12609)  /* ㅁ */) => { curr_state = 35; continue; },
                _ => { break 0; },
            },
            13 => match curr_char {
                Some(자모::초성(12609)  /* ㅁ */) => { curr_state = 36; continue; },
                _ => { break 0; },
            },
            14 => match curr_char {
                Some(자모::종성(12593)  /* ㄱ */) => {  break count - 2;  },
                Some(자모::종성(12594)  /* ㄲ */) => {  break count - 2;  },
                Some(자모::종성(12595)  /* ㄳ */) => {  break count - 2;  },
                Some(자모::종성(12596)  /* ㄴ */) => {  break count - 2;  },
                Some(자모::종성(12597)  /* ㄵ */) => {  break count - 2;  },
                Some(자모::종성(12598)  /* ㄶ */) => {  break count - 2;  },
                Some(자모::종성(12599)  /* ㄷ */) => {  break count - 2;  },
                Some(자모::종성(12601)  /* ㄹ */) => {  break count - 2;  },
                Some(자모::종성(12602)  /* ㄺ */) => {  break count - 2;  },
                Some(자모::종성(12603)  /* ㄻ */) => {  break count - 2;  },
                Some(자모::종성(12604)  /* ㄼ */) => {  break count - 2;  },
                Some(자모::종성(12605)  /* ㄽ */) => {  break count - 2;  },
                Some(자모::종성(12606)  /* ㄾ */) => {  break count - 2;  },
                Some(자모::종성(12607)  /* ㄿ */) => {  break count - 2;  },
                Some(자모::종성(12608)  /* ㅀ */) => {  break count - 2;  },
                Some(자모::종성(12609)  /* ㅁ */) => {  break count - 2;  },
                Some(자모::종성(12610)  /* ㅂ */) => {  break count - 2;  },
                Some(자모::종성(12612)  /* ㅄ */) => {  break count - 2;  },
                Some(자모::종성(12613)  /* ㅅ */) => {  break count - 2;  },
                Some(자모::종성(12614)  /* ㅆ */) => {  break count - 2;  },
                Some(자모::종성(12615)  /* ㅇ */) => {  break count - 2;  },
                Some(자모::종성(12616)  /* ㅈ */) => {  break count - 2;  },
                Some(자모::종성(12618)  /* ㅊ */) => {  break count - 2;  },
                Some(자모::종성(12619)  /* ㅋ */) => {  break count - 2;  },
                Some(자모::종성(12620)  /* ㅌ */) => {  break count - 2;  },
                Some(자모::종성(12621)  /* ㅍ */) => {  break count - 2;  },
                Some(자모::종성(12622)  /* ㅎ */) => {  break count - 2;  },
                _ => { break 0; },
            },
            15 => match curr_char {
                Some(자모::종성(12596)  /* ㄴ */) => { curr_state = 65; continue; },
                Some(자모::중성(12623)  /* ㅏ */) => { curr_state = 64; continue; },
                _ => { break 0; },
            },
            16 => match curr_char {
                Some(자모::초성(12601)  /* ㄹ */) => { curr_state = 67; continue; },
                Some(자모::초성(12615)  /* ㅇ */) => { curr_state = 66; continue; },
                _ => { break 0; },
            },
            17 => match curr_char {
                Some(자모::종성(12593)  /* ㄱ */) => {  break count - 2;  },
                Some(자모::종성(12594)  /* ㄲ */) => {  break count - 2;  },
                Some(자모::종성(12595)  /* ㄳ */) => {  break count - 2;  },
                Some(자모::종성(12596)  /* ㄴ */) => {  break count - 2;  },
                Some(자모::종성(12597)  /* ㄵ */) => {  break count - 2;  },
                Some(자모::종성(12598)  /* ㄶ */) => {  break count - 2;  },
                Some(자모::종성(12599)  /* ㄷ */) => {  break count - 2;  },
                Some(자모::종성(12601)  /* ㄹ */) => {  break count - 2;  },
                Some(자모::종성(12602)  /* ㄺ */) => {  break count - 2;  },
                Some(자모::종성(12603)  /* ㄻ */) => {  break count - 2;  },
                Some(자모::종성(12604)  /* ㄼ */) => {  break count - 2;  },
                Some(자모::종성(12605)  /* ㄽ */) => {  break count - 2;  },
                Some(자모::종성(12606)  /* ㄾ */) => {  break count - 2;  },
                Some(자모::종성(12607)  /* ㄿ */) => {  break count - 2;  },
                Some(자모::종성(12608)  /* ㅀ */) => {  break count - 2;  },
                Some(자모::종성(12609)  /* ㅁ */) => {  break count - 2;  },
                Some(자모::종성(12610)  /* ㅂ */) => {  break count - 2;  },
                Some(자모::종성(12612)  /* ㅄ */) => {  break count - 2;  },
                Some(자모::종성(12613)  /* ㅅ */) => {  break count - 2;  },
                Some(자모::종성(12614)  /* ㅆ */) => {  break count - 2;  },
                Some(자모::종성(12615)  /* ㅇ */) => {  break count - 2;  },
                Some(자모::종성(12616)  /* ㅈ */) => {  break count - 2;  },
                Some(자모::종성(12618)  /* ㅊ */) => {  break count - 2;  },
                Some(자모::종성(12619)  /* ㅋ */) => {  break count - 2;  },
                Some(자모::종성(12620)  /* ㅌ */) => {  break count - 2;  },
                Some(자모::종성(12621)  /* ㅍ */) => {  break count - 2;  },
                Some(자모::종성(12622)  /* ㅎ */) => {  break count - 2;  },
                _ => { break 0; },
            },
            18 => match curr_char {
                Some(자모::중성(12623)  /* ㅏ */) => {  break count - 2;  },
                Some(자모::중성(12624)  /* ㅐ */) => {  break count - 2;  },
                Some(자모::중성(12625)  /* ㅑ */) => {  break count - 2;  },
                Some(자모::중성(12626)  /* ㅒ */) => {  break count - 2;  },
                Some(자모::중성(12627)  /* ㅓ */) => {  break count - 2;  },
                Some(자모::중성(12628)  /* ㅔ */) => {  break count - 2;  },
                Some(자모::중성(12629)  /* ㅕ */) => {  break count - 2;  },
                Some(자모::중성(12630)  /* ㅖ */) => {  break count - 2;  },
                Some(자모::중성(12631)  /* ㅗ */) => {  break count - 2;  },
                Some(자모::중성(12632)  /* ㅘ */) => {  break count - 2;  },
                Some(자모::중성(12633)  /* ㅙ */) => {  break count - 2;  },
                Some(자모::중성(12634)  /* ㅚ */) => {  break count - 2;  },
                Some(자모::중성(12635)  /* ㅛ */) => {  break count - 2;  },
                Some(자모::중성(12636)  /* ㅜ */) => {  break count - 2;  },
                Some(자모::중성(12637)  /* ㅝ */) => {  break count - 2;  },
                Some(자모::중성(12638)  /* ㅞ */) => {  break count - 2;  },
                Some(자모::중성(12639)  /* ㅟ */) => {  break count - 2;  },
                Some(자모::중성(12640)  /* ㅠ */) => {  break count - 2;  },
                Some(자모::중성(12641)  /* ㅡ */) => {  break count - 2;  },
                Some(자모::중성(12642)  /* ㅢ */) => {  break count - 2;  },
                Some(자모::중성(12643)  /* ㅣ */) => {  break count - 2;  },
                _ => { break 0; },
            },
            19 => match curr_char {
                Some(자모::중성(12641)  /* ㅡ */) => { curr_state = 116; continue; },
                _ => { break count - 1; },
            },
            20 => match curr_char {
                Some(자모::중성(12623)  /* ㅏ */) => { curr_state = 117; continue; },
                Some(자모::중성(12643)  /* ㅣ */) => { curr_state = 118; continue; },
                _ => { break 0; },
            },
            22 => match curr_char {
                Some(자모::초성(12601)  /* ㄹ */) => { curr_state = 119; continue; },
                _ => { break 0; },
            },
            23 => match curr_char {
                Some(자모::중성(12623)  /* ㅏ */) => {  break count - 2;  },
                Some(자모::중성(12624)  /* ㅐ */) => {  break count - 2;  },
                Some(자모::중성(12625)  /* ㅑ */) => {  break count - 2;  },
                Some(자모::중성(12626)  /* ㅒ */) => {  break count - 2;  },
                Some(자모::중성(12627)  /* ㅓ */) => {  break count - 2;  },
                Some(자모::중성(12628)  /* ㅔ */) => {  break count - 2;  },
                Some(자모::중성(12629)  /* ㅕ */) => {  break count - 2;  },
                Some(자모::중성(12630)  /* ㅖ */) => {  break count - 2;  },
                Some(자모::중성(12631)  /* ㅗ */) => {  break count - 2;  },
                Some(자모::중성(12632)  /* ㅘ */) => {  break count - 2;  },
                Some(자모::중성(12633)  /* ㅙ */) => {  break count - 2;  },
                Some(자모::중성(12634)  /* ㅚ */) => {  break count - 2;  },
                Some(자모::중성(12635)  /* ㅛ */) => {  break count - 2;  },
                Some(자모::중성(12636)  /* ㅜ */) => {  break count - 2;  },
                Some(자모::중성(12637)  /* ㅝ */) => {  break count - 2;  },
                Some(자모::중성(12638)  /* ㅞ */) => {  break count - 2;  },
                Some(자모::중성(12639)  /* ㅟ */) => {  break count - 2;  },
                Some(자모::중성(12640)  /* ㅠ */) => {  break count - 2;  },
                Some(자모::중성(12641)  /* ㅡ */) => {  break count - 2;  },
                Some(자모::중성(12642)  /* ㅢ */) => {  break count - 2;  },
                Some(자모::중성(12643)  /* ㅣ */) => {  break count - 2;  },
                _ => { break 0; },
            },
            24 => match curr_char {
                Some(자모::중성(12623)  /* ㅏ */) => { curr_state = 142; continue; },
                Some(자모::중성(12643)  /* ㅣ */) => { curr_state = 141; continue; },
                _ => { break 0; },
            },
            27 => match curr_char {
                Some(자모::중성(12628)  /* ㅔ */) => { curr_state = 143; continue; },
                _ => { break 0; },
            },
            28 => match curr_char {
                Some(자모::종성(12596)  /* ㄴ */) => { curr_state = 144; continue; },
                _ => { break 0; },
            },
            30 => match curr_char {
                Some(자모::종성(12596)  /* ㄴ */) => { curr_state = 145; continue; },
                _ => { break 0; },
            },
            31 => match curr_char {
                Some(자모::중성(12628)  /* ㅔ */) => { curr_state = 146; continue; },
                _ => { break 0; },
            },
            32 => match curr_char {
                Some(자모::중성(12636)  /* ㅜ */) => { curr_state = 147; continue; },
                _ => { break 0; },
            },
            33 => match curr_char {
                Some(자모::종성(12593)  /* ㄱ */) => {  break count - 2;  },
                Some(자모::종성(12594)  /* ㄲ */) => {  break count - 2;  },
                Some(자모::종성(12595)  /* ㄳ */) => {  break count - 2;  },
                Some(자모::종성(12596)  /* ㄴ */) => {  break count - 2;  },
                Some(자모::종성(12597)  /* ㄵ */) => {  break count - 2;  },
                Some(자모::종성(12598)  /* ㄶ */) => {  break count - 2;  },
                Some(자모::종성(12599)  /* ㄷ */) => {  break count - 2;  },
                Some(자모::종성(12601)  /* ㄹ */) => {  break count - 2;  },
                Some(자모::종성(12602)  /* ㄺ */) => {  break count - 2;  },
                Some(자모::종성(12603)  /* ㄻ */) => {  break count - 2;  },
                Some(자모::종성(12604)  /* ㄼ */) => {  break count - 2;  },
                Some(자모::종성(12605)  /* ㄽ */) => {  break count - 2;  },
                Some(자모::종성(12606)  /* ㄾ */) => {  break count - 2;  },
                Some(자모::종성(12607)  /* ㄿ */) => {  break count - 2;  },
                Some(자모::종성(12608)  /* ㅀ */) => {  break count - 2;  },
                Some(자모::종성(12609)  /* ㅁ */) => {  break count - 2;  },
                Some(자모::종성(12610)  /* ㅂ */) => {  break count - 2;  },
                Some(자모::종성(12612)  /* ㅄ */) => {  break count - 2;  },
                Some(자모::종성(12613)  /* ㅅ */) => {  break count - 2;  },
                Some(자모::종성(12614)  /* ㅆ */) => {  break count - 2;  },
                Some(자모::종성(12615)  /* ㅇ */) => {  break count - 2;  },
                Some(자모::종성(12616)  /* ㅈ */) => {  break count - 2;  },
                Some(자모::종성(12618)  /* ㅊ */) => {  break count - 2;  },
                Some(자모::종성(12619)  /* ㅋ */) => {  break count - 2;  },
                Some(자모::종성(12620)  /* ㅌ */) => {  break count - 2;  },
                Some(자모::종성(12621)  /* ㅍ */) => {  break count - 2;  },
                Some(자모::종성(12622)  /* ㅎ */) => {  break count - 2;  },
                _ => { break 0; },
            },
            34 => match curr_char {
                Some(자모::중성(12623)  /* ㅏ */) => {  break count - 2;  },
                Some(자모::중성(12624)  /* ㅐ */) => {  break count - 2;  },
                Some(자모::중성(12625)  /* ㅑ */) => {  break count - 2;  },
                Some(자모::중성(12626)  /* ㅒ */) => {  break count - 2;  },
                Some(자모::중성(12627)  /* ㅓ */) => {  break count - 2;  },
                Some(자모::중성(12628)  /* ㅔ */) => {  break count - 2;  },
                Some(자모::중성(12629)  /* ㅕ */) => {  break count - 2;  },
                Some(자모::중성(12630)  /* ㅖ */) => {  break count - 2;  },
                Some(자모::중성(12631)  /* ㅗ */) => {  break count - 2;  },
                Some(자모::중성(12632)  /* ㅘ */) => {  break count - 2;  },
                Some(자모::중성(12633)  /* ㅙ */) => {  break count - 2;  },
                Some(자모::중성(12634)  /* ㅚ */) => {  break count - 2;  },
                Some(자모::중성(12635)  /* ㅛ */) => {  break count - 2;  },
                Some(자모::중성(12636)  /* ㅜ */) => {  break count - 2;  },
                Some(자모::중성(12637)  /* ㅝ */) => {  break count - 2;  },
                Some(자모::중성(12638)  /* ㅞ */) => {  break count - 2;  },
                Some(자모::중성(12639)  /* ㅟ */) => {  break count - 2;  },
                Some(자모::중성(12640)  /* ㅠ */) => {  break count - 2;  },
                Some(자모::중성(12641)  /* ㅡ */) => {  break count - 2;  },
                Some(자모::중성(12642)  /* ㅢ */) => {  break count - 2;  },
                Some(자모::중성(12643)  /* ㅣ */) => {  break count - 2;  },
                _ => { break 0; },
            },
            35 => match curr_char {
                Some(자모::중성(12643)  /* ㅣ */) => { curr_state = 196; continue; },
                _ => { break count - 1; },
            },
            36 => match curr_char {
                Some(자모::중성(12623)  /* ㅏ */) => { curr_state = 198; continue; },
                Some(자모::중성(12643)  /* ㅣ */) => { curr_state = 197; continue; },
                _ => { break 0; },
            },
            64 => match curr_char {
                Some(자모::초성(12594)  /* ㄲ */) => {  break count - 1;  },
                _ => { break 0; },
            },
            65 => match curr_char {
                Some(자모::중성(12623)  /* ㅏ */) => { curr_state = 201; continue; },
                Some(자모::중성(12643)  /* ㅣ */) => { curr_state = 200; continue; },
                _ => { break 0; },
            },
            66 => match curr_char {
                Some(자모::종성(12593)  /* ㄱ */) => {  break count - 2;  },
                Some(자모::종성(12594)  /* ㄲ */) => {  break count - 2;  },
                Some(자모::종성(12595)  /* ㄳ */) => {  break count - 2;  },
                Some(자모::종성(12596)  /* ㄴ */) => {  break count - 2;  },
                Some(자모::종성(12597)  /* ㄵ */) => {  break count - 2;  },
                Some(자모::종성(12598)  /* ㄶ */) => {  break count - 2;  },
                Some(자모::종성(12599)  /* ㄷ */) => {  break count - 2;  },
                Some(자모::종성(12601)  /* ㄹ */) => {  break count - 2;  },
                Some(자모::종성(12602)  /* ㄺ */) => {  break count - 2;  },
                Some(자모::종성(12603)  /* ㄻ */) => {  break count - 2;  },
                Some(자모::종성(12604)  /* ㄼ */) => {  break count - 2;  },
                Some(자모::종성(12605)  /* ㄽ */) => {  break count - 2;  },
                Some(자모::종성(12606)  /* ㄾ */) => {  break count - 2;  },
                Some(자모::종성(12607)  /* ㄿ */) => {  break count - 2;  },
                Some(자모::종성(12608)  /* ㅀ */) => {  break count - 2;  },
                Some(자모::종성(12609)  /* ㅁ */) => {  break count - 2;  },
                Some(자모::종성(12610)  /* ㅂ */) => {  break count - 2;  },
                Some(자모::종성(12612)  /* ㅄ */) => {  break count - 2;  },
                Some(자모::종성(12613)  /* ㅅ */) => {  break count - 2;  },
                Some(자모::종성(12614)  /* ㅆ */) => {  break count - 2;  },
                Some(자모::종성(12615)  /* ㅇ */) => {  break count - 2;  },
                Some(자모::종성(12616)  /* ㅈ */) => {  break count - 2;  },
                Some(자모::종성(12618)  /* ㅊ */) => {  break count - 2;  },
                Some(자모::종성(12619)  /* ㅋ */) => {  break count - 2;  },
                Some(자모::종성(12620)  /* ㅌ */) => {  break count - 2;  },
                Some(자모::종성(12621)  /* ㅍ */) => {  break count - 2;  },
                Some(자모::종성(12622)  /* ㅎ */) => {  break count - 2;  },
                _ => { break 0; },
            },
            67 => match curr_char {
                Some(자모::중성(12623)  /* ㅏ */) => {  break count - 2;  },
                Some(자모::중성(12624)  /* ㅐ */) => {  break count - 2;  },
                Some(자모::중성(12625)  /* ㅑ */) => {  break count - 2;  },
                Some(자모::중성(12626)  /* ㅒ */) => {  break count - 2;  },
                Some(자모::중성(12627)  /* ㅓ */) => {  break count - 2;  },
                Some(자모::중성(12628)  /* ㅔ */) => {  break count - 2;  },
                Some(자모::중성(12629)  /* ㅕ */) => {  break count - 2;  },
                Some(자모::중성(12630)  /* ㅖ */) => {  break count - 2;  },
                Some(자모::중성(12631)  /* ㅗ */) => {  break count - 2;  },
                Some(자모::중성(12632)  /* ㅘ */) => {  break count - 2;  },
                Some(자모::중성(12633)  /* ㅙ */) => {  break count - 2;  },
                Some(자모::중성(12634)  /* ㅚ */) => {  break count - 2;  },
                Some(자모::중성(12635)  /* ㅛ */) => {  break count - 2;  },
                Some(자모::중성(12636)  /* ㅜ */) => {  break count - 2;  },
                Some(자모::중성(12637)  /* ㅝ */) => {  break count - 2;  },
                Some(자모::중성(12638)  /* ㅞ */) => {  break count - 2;  },
                Some(자모::중성(12639)  /* ㅟ */) => {  break count - 2;  },
                Some(자모::중성(12640)  /* ㅠ */) => {  break count - 2;  },
                Some(자모::중성(12641)  /* ㅡ */) => {  break count - 2;  },
                Some(자모::중성(12642)  /* ㅢ */) => {  break count - 2;  },
                Some(자모::중성(12643)  /* ㅣ */) => {  break count - 2;  },
                _ => { break 0; },
            },
            116 => match curr_char {
                Some(자모::초성(12615)  /* ㅇ */) => { curr_state = 250; continue; },
                _ => { break 0; },
            },
            117 => match curr_char {
                Some(자모::초성(12601)  /* ㄹ */) => { curr_state = 251; continue; },
                Some(자모::초성(12622)  /* ㅎ */) => {  break count - 1;  },
                _ => { break 0; },
            },
            118 => match curr_char {
                Some(자모::초성(12615)  /* ㅇ */) => {  break count - 1;  },
                _ => { break 0; },
            },
            119 => match curr_char {
                Some(자모::중성(12623)  /* ㅏ */) => {  break count - 2;  },
                Some(자모::중성(12624)  /* ㅐ */) => {  break count - 2;  },
                Some(자모::중성(12625)  /* ㅑ */) => {  break count - 2;  },
                Some(자모::중성(12626)  /* ㅒ */) => {  break count - 2;  },
                Some(자모::중성(12627)  /* ㅓ */) => {  break count - 2;  },
                Some(자모::중성(12628)  /* ㅔ */) => {  break count - 2;  },
                Some(자모::중성(12629)  /* ㅕ */) => {  break count - 2;  },
                Some(자모::중성(12630)  /* ㅖ */) => {  break count - 2;  },
                Some(자모::중성(12631)  /* ㅗ */) => {  break count - 2;  },
                Some(자모::중성(12632)  /* ㅘ */) => {  break count - 2;  },
                Some(자모::중성(12633)  /* ㅙ */) => {  break count - 2;  },
                Some(자모::중성(12634)  /* ㅚ */) => {  break count - 2;  },
                Some(자모::중성(12635)  /* ㅛ */) => {  break count - 2;  },
                Some(자모::중성(12636)  /* ㅜ */) => {  break count - 2;  },
                Some(자모::중성(12637)  /* ㅝ */) => {  break count - 2;  },
                Some(자모::중성(12638)  /* ㅞ */) => {  break count - 2;  },
                Some(자모::중성(12639)  /* ㅟ */) => {  break count - 2;  },
                Some(자모::중성(12640)  /* ㅠ */) => {  break count - 2;  },
                Some(자모::중성(12641)  /* ㅡ */) => {  break count - 2;  },
                Some(자모::중성(12642)  /* ㅢ */) => {  break count - 2;  },
                Some(자모::중성(12643)  /* ㅣ */) => { curr_state = 254; continue; },
                _ => { break 0; },
            },
            141 => match curr_char {
                Some(자모::초성(12596)  /* ㄴ */) => { curr_state = 275; continue; },
                Some(자모::초성(12615)  /* ㅇ */) => {  break count - 1;  },
                _ => { break 0; },
            },
            142 => match curr_char {
                Some(자모::초성(12622)  /* ㅎ */) => {  break count - 1;  },
                _ => { break 0; },
            },
            143 => match curr_char {
                Some(자모::초성(12615)  /* ㅇ */) => {  break count - 1;  },
                _ => { break 0; },
            },
            144 => match curr_char {
                Some(자모::중성(12623)  /* ㅏ */) => { curr_state = 279; continue; },
                _ => { break 0; },
            },
            145 => match curr_char {
                Some(자모::중성(12623)  /* ㅏ */) => { curr_state = 281; continue; },
                Some(자모::중성(12643)  /* ㅣ */) => { curr_state = 280; continue; },
                _ => { break 0; },
            },
            146 => match curr_char {
                Some(자모::초성(12615)  /* ㅇ */) => {  break count - 1;  },
                _ => { break 0; },
            },
            147 => match curr_char {
                Some(자모::초성(12610)  /* ㅂ */) => {  break count - 1;  },
                _ => { break 0; },
            },
            196 => match curr_char {
                Some(자모::초성(12616)  /* ㅈ */) => { curr_state = 284; continue; },
                _ => { break 0; },
            },
            197 => match curr_char {
                Some(자모::초성(12615)  /* ㅇ */) => {  break count - 1;  },
                _ => { break 0; },
            },
            198 => match curr_char {
                Some(자모::초성(12622)  /* ㅎ */) => {  break count - 1;  },
                _ => { break 0; },
            },
            200 => match curr_char {
                Some(자모::초성(12615)  /* ㅇ */) => {  break count - 1;  },
                _ => { break 0; },
            },
            201 => match curr_char {
                Some(자모::초성(12622)  /* ㅎ */) => {  break count - 1;  },
                _ => { break 0; },
            },
            250 => match curr_char {
                Some(자모::종성(12593)  /* ㄱ */) => {  break count - 2;  },
                Some(자모::종성(12594)  /* ㄲ */) => {  break count - 2;  },
                Some(자모::종성(12595)  /* ㄳ */) => {  break count - 2;  },
                Some(자모::종성(12596)  /* ㄴ */) => {  break count - 2;  },
                Some(자모::종성(12597)  /* ㄵ */) => {  break count - 2;  },
                Some(자모::종성(12598)  /* ㄶ */) => {  break count - 2;  },
                Some(자모::종성(12599)  /* ㄷ */) => {  break count - 2;  },
                Some(자모::종성(12601)  /* ㄹ */) => {  break count - 2;  },
                Some(자모::종성(12602)  /* ㄺ */) => {  break count - 2;  },
                Some(자모::종성(12603)  /* ㄻ */) => {  break count - 2;  },
                Some(자모::종성(12604)  /* ㄼ */) => {  break count - 2;  },
                Some(자모::종성(12605)  /* ㄽ */) => {  break count - 2;  },
                Some(자모::종성(12606)  /* ㄾ */) => {  break count - 2;  },
                Some(자모::종성(12607)  /* ㄿ */) => {  break count - 2;  },
                Some(자모::종성(12608)  /* ㅀ */) => {  break count - 2;  },
                Some(자모::종성(12609)  /* ㅁ */) => {  break count - 2;  },
                Some(자모::종성(12610)  /* ㅂ */) => {  break count - 2;  },
                Some(자모::종성(12612)  /* ㅄ */) => {  break count - 2;  },
                Some(자모::종성(12613)  /* ㅅ */) => {  break count - 2;  },
                Some(자모::종성(12614)  /* ㅆ */) => {  break count - 2;  },
                Some(자모::종성(12615)  /* ㅇ */) => {  break count - 2;  },
                Some(자모::종성(12616)  /* ㅈ */) => {  break count - 2;  },
                Some(자모::종성(12618)  /* ㅊ */) => {  break count - 2;  },
                Some(자모::종성(12619)  /* ㅋ */) => {  break count - 2;  },
                Some(자모::종성(12620)  /* ㅌ */) => {  break count - 2;  },
                Some(자모::종성(12621)  /* ㅍ */) => {  break count - 2;  },
                Some(자모::종성(12622)  /* ㅎ */) => {  break count - 2;  },
                _ => { break 0; },
            },
            251 => match curr_char {
                Some(자모::중성(12623)  /* ㅏ */) => {  break count - 2;  },
                Some(자모::중성(12624)  /* ㅐ */) => {  break count - 2;  },
                Some(자모::중성(12625)  /* ㅑ */) => {  break count - 2;  },
                Some(자모::중성(12626)  /* ㅒ */) => {  break count - 2;  },
                Some(자모::중성(12627)  /* ㅓ */) => {  break count - 2;  },
                Some(자모::중성(12628)  /* ㅔ */) => {  break count - 2;  },
                Some(자모::중성(12629)  /* ㅕ */) => {  break count - 2;  },
                Some(자모::중성(12630)  /* ㅖ */) => {  break count - 2;  },
                Some(자모::중성(12631)  /* ㅗ */) => {  break count - 2;  },
                Some(자모::중성(12632)  /* ㅘ */) => {  break count - 2;  },
                Some(자모::중성(12633)  /* ㅙ */) => {  break count - 2;  },
                Some(자모::중성(12634)  /* ㅚ */) => {  break count - 2;  },
                Some(자모::중성(12635)  /* ㅛ */) => {  break count - 2;  },
                Some(자모::중성(12636)  /* ㅜ */) => {  break count - 2;  },
                Some(자모::중성(12637)  /* ㅝ */) => {  break count - 2;  },
                Some(자모::중성(12638)  /* ㅞ */) => {  break count - 2;  },
                Some(자모::중성(12639)  /* ㅟ */) => {  break count - 2;  },
                Some(자모::중성(12640)  /* ㅠ */) => {  break count - 2;  },
                Some(자모::중성(12641)  /* ㅡ */) => {  break count - 2;  },
                Some(자모::중성(12642)  /* ㅢ */) => {  break count - 2;  },
                Some(자모::중성(12643)  /* ㅣ */) => { curr_state = 316; continue; },
                _ => { break 0; },
            },
            254 => match curr_char {
                Some(자모::초성(12615)  /* ㅇ */) => { curr_state = 337; continue; },
                _ => { break count - 2; },
            },
            275 => match curr_char {
                Some(자모::종성(12610)  /* ㅂ */) => { curr_state = 338; continue; },
                _ => { break 0; },
            },
            279 => match curr_char {
                Some(자모::초성(12622)  /* ㅎ */) => {  break count - 1;  },
                _ => { break 0; },
            },
            280 => match curr_char {
                Some(자모::초성(12615)  /* ㅇ */) => {  break count - 1;  },
                _ => { break 0; },
            },
            281 => match curr_char {
                Some(자모::초성(12622)  /* ㅎ */) => {  break count - 1;  },
                _ => { break 0; },
            },
            284 => match curr_char {
                Some(자모::중성(12623)  /* ㅏ */) => { curr_state = 343; continue; },
                Some(자모::중성(12643)  /* ㅣ */) => { curr_state = 342; continue; },
                _ => { break 0; },
            },
            316 => match curr_char {
                Some(자모::초성(12615)  /* ㅇ */) => { curr_state = 344; continue; },
                _ => { break count - 2; },
            },
            337 => match curr_char {
                Some(자모::종성(12593)  /* ㄱ */) => {  break count - 2;  },
                Some(자모::종성(12594)  /* ㄲ */) => {  break count - 2;  },
                Some(자모::종성(12595)  /* ㄳ */) => {  break count - 2;  },
                Some(자모::종성(12596)  /* ㄴ */) => {  break count - 2;  },
                Some(자모::종성(12597)  /* ㄵ */) => {  break count - 2;  },
                Some(자모::종성(12598)  /* ㄶ */) => {  break count - 2;  },
                Some(자모::종성(12599)  /* ㄷ */) => {  break count - 2;  },
                Some(자모::종성(12601)  /* ㄹ */) => {  break count - 2;  },
                Some(자모::종성(12602)  /* ㄺ */) => {  break count - 2;  },
                Some(자모::종성(12603)  /* ㄻ */) => {  break count - 2;  },
                Some(자모::종성(12604)  /* ㄼ */) => {  break count - 2;  },
                Some(자모::종성(12605)  /* ㄽ */) => {  break count - 2;  },
                Some(자모::종성(12606)  /* ㄾ */) => {  break count - 2;  },
                Some(자모::종성(12607)  /* ㄿ */) => {  break count - 2;  },
                Some(자모::종성(12608)  /* ㅀ */) => {  break count - 2;  },
                Some(자모::종성(12609)  /* ㅁ */) => {  break count - 2;  },
                Some(자모::종성(12610)  /* ㅂ */) => {  break count - 2;  },
                Some(자모::종성(12612)  /* ㅄ */) => {  break count - 2;  },
                Some(자모::종성(12613)  /* ㅅ */) => {  break count - 2;  },
                Some(자모::종성(12614)  /* ㅆ */) => {  break count - 2;  },
                Some(자모::종성(12615)  /* ㅇ */) => {  break count - 2;  },
                Some(자모::종성(12616)  /* ㅈ */) => {  break count - 2;  },
                Some(자모::종성(12618)  /* ㅊ */) => {  break count - 2;  },
                Some(자모::종성(12619)  /* ㅋ */) => {  break count - 2;  },
                Some(자모::종성(12620)  /* ㅌ */) => {  break count - 2;  },
                Some(자모::종성(12621)  /* ㅍ */) => {  break count - 2;  },
                Some(자모::종성(12622)  /* ㅎ */) => {  break count - 2;  },
                _ => { break 0; },
            },
            338 => match curr_char {
                Some(자모::중성(12623)  /* ㅏ */) => { curr_state = 373; continue; },
                Some(자모::중성(12643)  /* ㅣ */) => { curr_state = 372; continue; },
                _ => { break 0; },
            },
            342 => match curr_char {
                Some(자모::초성(12615)  /* ㅇ */) => {  break count - 1;  },
                _ => { break 0; },
            },
            343 => match curr_char {
                Some(자모::초성(12622)  /* ㅎ */) => {  break count - 1;  },
                _ => { break 0; },
            },
            344 => match curr_char {
                Some(자모::종성(12593)  /* ㄱ */) => {  break count - 2;  },
                Some(자모::종성(12594)  /* ㄲ */) => {  break count - 2;  },
                Some(자모::종성(12595)  /* ㄳ */) => {  break count - 2;  },
                Some(자모::종성(12596)  /* ㄴ */) => {  break count - 2;  },
                Some(자모::종성(12597)  /* ㄵ */) => {  break count - 2;  },
                Some(자모::종성(12598)  /* ㄶ */) => {  break count - 2;  },
                Some(자모::종성(12599)  /* ㄷ */) => {  break count - 2;  },
                Some(자모::종성(12601)  /* ㄹ */) => {  break count - 2;  },
                Some(자모::종성(12602)  /* ㄺ */) => {  break count - 2;  },
                Some(자모::종성(12603)  /* ㄻ */) => {  break count - 2;  },
                Some(자모::종성(12604)  /* ㄼ */) => {  break count - 2;  },
                Some(자모::종성(12605)  /* ㄽ */) => {  break count - 2;  },
                Some(자모::종성(12606)  /* ㄾ */) => {  break count - 2;  },
                Some(자모::종성(12607)  /* ㄿ */) => {  break count - 2;  },
                Some(자모::종성(12608)  /* ㅀ */) => {  break count - 2;  },
                Some(자모::종성(12609)  /* ㅁ */) => {  break count - 2;  },
                Some(자모::종성(12610)  /* ㅂ */) => {  break count - 2;  },
                Some(자모::종성(12612)  /* ㅄ */) => {  break count - 2;  },
                Some(자모::종성(12613)  /* ㅅ */) => {  break count - 2;  },
                Some(자모::종성(12614)  /* ㅆ */) => {  break count - 2;  },
                Some(자모::종성(12615)  /* ㅇ */) => {  break count - 2;  },
                Some(자모::종성(12616)  /* ㅈ */) => {  break count - 2;  },
                Some(자모::종성(12618)  /* ㅊ */) => {  break count - 2;  },
                Some(자모::종성(12619)  /* ㅋ */) => {  break count - 2;  },
                Some(자모::종성(12620)  /* ㅌ */) => {  break count - 2;  },
                Some(자모::종성(12621)  /* ㅍ */) => {  break count - 2;  },
                Some(자모::종성(12622)  /* ㅎ */) => {  break count - 2;  },
                _ => { break 0; },
            },
            372 => match curr_char {
                Some(자모::초성(12615)  /* ㅇ */) => {  break count - 1;  },
                _ => { break 0; },
            },
            373 => match curr_char {
                Some(자모::초성(12622)  /* ㅎ */) => {  break count - 1;  },
                _ => { break 0; },
            },
            _ => unreachable!(),
        }
    };

    assemble(&자모s[..(자모s.len() - cut_at)].to_vec())
}
