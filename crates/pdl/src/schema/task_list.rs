use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    static ref TASK_LIST_RE: Regex = Regex::new(r"^(\s*)(\+|\*|\-)\s*\[.\]\s+.+$").unwrap();
    static ref EMPTY_RE: Regex = Regex::new(r"^\s*$").unwrap();
}

// It assumes that `try_extract_task_list(s)` returns `Ok(_)`.
pub fn count_task_list_elements(s: &str) -> usize {
    let mut count = 0;

    for line in s.lines() {
        if let Some(cap) = TASK_LIST_RE.captures(line) {
            // it only counts top-level tasks
            if cap.get(1).unwrap().is_empty() {
                count += 1;
            }
        }
    }

    count
}

enum ParseState {
    NotSeenTaskListYet,
    LookingAtTaskList {
        marker: char,

        // The spec allows each item in task list to be followed by an arbitrary
        // length paragraph. But that would be too tough to parse, right? So it
        // allows at most 1 line to follow a task list item.
        this_line_must_be_another_task: bool,
    },
    AfterTaskList,
}

// https://github.github.com/gfm/#task-list-items-extension-
// https://github.github.com/gfm/#list-items
//
// 1. It doesn't care about code fences. I hope LLMs are smart enough to know that
//    they should not mix code fences and task lists.
// 2. Its syntax is a bit stricter than gfm. Instead, it carefully instructs LLMs to
//    follow the rules.
pub fn try_extract_task_list(s: &str) -> Result<String, String> {
    let mut buffer = vec![];
    let mut curr_state = ParseState::NotSeenTaskListYet;

    for line in s.lines() {
        match &mut curr_state {
            ParseState::NotSeenTaskListYet => {
                if let Some(cap) = TASK_LIST_RE.captures(line) {
                    buffer.push(line);
                    curr_state = ParseState::LookingAtTaskList {
                        marker: cap.get(2).unwrap().as_str().chars().next().unwrap(),
                        this_line_must_be_another_task: false,
                    };
                }
            },
            ParseState::LookingAtTaskList { marker, this_line_must_be_another_task } => {
                if EMPTY_RE.is_match(line) {
                    curr_state = ParseState::AfterTaskList;
                }

                else if let Some(cap) = TASK_LIST_RE.captures(line) {
                    let curr_marker = cap.get(2).unwrap().as_str().chars().next().unwrap();

                    if curr_marker != *marker {
                        return Err(format!("I see multiple task lists in your output, each having different markers: `{curr_marker}` and `{marker}`. Please give me exactly one task list."));
                    }

                    buffer.push(line);
                }

                else if *this_line_must_be_another_task {
                    curr_state = ParseState::AfterTaskList;
                }

                else {
                    *this_line_must_be_another_task = true;
                    buffer.push(line);
                }
            },
            ParseState::AfterTaskList => {
                if TASK_LIST_RE.is_match(line) {
                    return Err(String::from("I see multiple task lists in your output. Please give me exactly one task list."));
                }
            },
        }
    }

    if buffer.is_empty() {
        Err(String::from("I cannot find a task list in your output. Please give me a markdown task list. A task list consists of lines where each line starts with \"-\", followed by a whitespace, followed by \"[ ]\" (if the task is not complete) or \"[X]\" (if the task is complete), and followed by a task."))
    }

    else {
        Ok(buffer.join("\n"))
    }
}
