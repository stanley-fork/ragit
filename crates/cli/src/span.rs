#[derive(Clone, Debug)]
pub enum Span {
    Exact(usize),  // including flags and args
    FirstArg,
    End,
    NthArg(usize),  // including args, not including flags
    Rendered((String, usize, usize)),
}

impl Span {
    pub(crate) fn render(&self, args: &[String]) -> Self {
        let mut rendered_args = Vec::with_capacity(args.len());
        let mut arg_indices = vec![];

        for (index, arg) in args.iter().enumerate() {
            if !arg.starts_with("--") {
                arg_indices.push(index);
            }

            if arg.contains(" ") || arg.contains("\"") || arg.contains("'") || arg.contains("\n") {
                rendered_args.push(format!("{arg:?}"));
            }

            else {
                rendered_args.push(arg.to_string());
            }
        }

        let selected_index = match self {
            Span::Exact(n) => *n,
            Span::FirstArg => match arg_indices.get(0) {
                Some(n) => *n,
                None => 0,
            },
            Span::NthArg(n) => match arg_indices.get(*n) {
                Some(n) => *n,
                None => 0,
            },
            _ => 0,
        };
        let mut joined_args = rendered_args.join(" ");
        let (start, end) = if joined_args.is_empty() {
            joined_args = String::from(" ");
            (0, 1)
        } else {
            match self {
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
