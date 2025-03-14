enum ParseState {
    BeforeFence,
    InFence(usize),
    AfterFence,
}

pub fn try_extract_code_fence(s: &str) -> Result<String, String> {
    let mut state = ParseState::BeforeFence;
    let mut code_lines = vec![];

    for line in s.lines() {
        match state {
            ParseState::BeforeFence => {
                if let Some(n) = count_opening_fence(line) {
                    state = ParseState::InFence(n);
                }
            },
            ParseState::InFence(fence_len) => {
                if let Some(n) = count_closing_fence(line) {
                    if n >= fence_len {
                        state = ParseState::AfterFence;
                    }

                    else {
                        code_lines.push(line.to_string());
                    }
                }

                else {
                    code_lines.push(line.to_string());
                }
            },
            ParseState::AfterFence => {
                if count_opening_fence(line).is_some() {
                    return Err(String::from("It seems like your response has more than 1 code block. Please give me exactly 1 code block."));
                }
            },
        }
    }

    if let ParseState::BeforeFence = state {
        Err(String::from("I cannot find a code block in your response. Please give me a fenced code block. An opening and closing fence consist of 3 or more backtick characters."))
    }

    // [spec](https://github.github.com/gfm/#fenced-code-blocks) allows omitting a closing fence
    else {
        let result = code_lines.join("\n");
        Ok(result.trim().to_string())
    }
}

fn count_opening_fence(line: &str) -> Option<usize> {
    let mut backtick_count = 0;
    let mut no_more_backticks = false;

    for c in line.chars() {
        match c {
            '`' => {
                if no_more_backticks {
                    return None;
                }

                else {
                    backtick_count += 1;
                }
            },
            'a'..='z' | 'A'..='Z' | '0'..='9' => {
                if backtick_count < 3 {
                    return None;
                }

                no_more_backticks = true;
            },
            ' ' => match backtick_count {
                0 => {},
                1..=2 => { return None; },
                3.. => {
                    no_more_backticks = true;
                },
            },
            _ => {
                return None;
            },
        }
    }

    if backtick_count >= 3 {
        Some(backtick_count)
    }

    else {
        None
    }
}

fn count_closing_fence(line: &str) -> Option<usize> {
    let mut backtick_count = 0;
    let mut no_more_backticks = false;

    for c in line.chars() {
        match c {
            '`' => {
                if no_more_backticks {
                    return None;
                }

                else {
                    backtick_count += 1;
                }
            },
            ' ' => match backtick_count {
                0 => {},
                1 | 2 => { return None; },
                3.. => { no_more_backticks = true; },
            },
            _ => {
                return None;
            },
        }
    }

    if backtick_count >= 3 {
        Some(backtick_count)
    }

    else {
        None
    }
}
