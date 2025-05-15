#[derive(Clone, Debug)]
pub enum Span {
    Exact(usize),  // including flags and args
    FirstArg,
    End,
    NthArg(usize),  // including args, not including flags
    Rendered((String, usize, usize)),
}

impl Span {
    pub fn render(&self, args: &[String], skip_first_n: usize) -> Self {
        let mut rendered_args = Vec::with_capacity(args.len());
        let mut arg_indices = vec![];

        for (index, arg) in args.iter().enumerate() {
            if !arg.starts_with("--") && index >= skip_first_n {
                arg_indices.push(index);
            }

            if arg.contains(" ") || arg.contains("\"") || arg.contains("'") || arg.contains("\n") {
                rendered_args.push(format!("{arg:?}"));
            }

            else {
                rendered_args.push(arg.to_string());
            }
        }

        let new_span = match self {
            Span::Exact(n) => Span::Exact(*n),
            Span::FirstArg => match arg_indices.get(0) {
                Some(n) => Span::Exact(*n),
                None => Span::End,
            },
            Span::NthArg(n) => match arg_indices.get(*n) {
                Some(n) => Span::Exact(*n),
                None => Span::End,
            },
            _ => self.clone(),
        };
        let selected_index = match new_span {
            Span::Exact(n) => n,
            _ => 0,
        };
        let mut joined_args = rendered_args.join(" ");
        let (start, end) = if joined_args.is_empty() {
            joined_args = String::from(" ");
            (0, 1)
        } else {
            // append a whitespace so that `Span::End` is more readable
            joined_args = format!("{joined_args} ");

            match new_span {
                Span::End => (joined_args.len() - 1, joined_args.len()),
                _ => (
                    rendered_args[..selected_index].iter().map(|arg| arg.len()).sum::<usize>() + selected_index,
                    rendered_args[..(selected_index + 1)].iter().map(|arg| arg.len()).sum::<usize>() + selected_index,
                ),
            }
        };

        Span::Rendered((
            joined_args,
            start,
            end,
        ))
    }

    pub fn unwrap_rendered(&self) -> (String, usize, usize) {
        match self {
            Span::Rendered((span, start, end)) => (span.to_string(), *start, *end),
            _ => panic!(),
        }
    }
}

pub fn underline_span(args: &str, start: usize, end: usize) -> String {
    format!(
        "{args}\n{}{}{}",
        " ".repeat(start),
        "^".repeat(end - start),
        " ".repeat(args.len() - end),
    )
}
