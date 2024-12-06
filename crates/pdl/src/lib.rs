// 1. render tera
// 2. line parser: <|user|>, <|system|>, <|assistant|>, <|schema|>
// 3. content parser: <|raw_media(ext:bytes)|>, <|media(path)|>

pub fn parse(
    s: &str,
    strict_mode: bool,
    context: &tera::Context,
) -> Result<_, Error> {
    let tera_rendered = tera::Tera::one_off(s, context, true) {
        Ok(t) => t,
        _ => if strict_mode {
            return _;
        } else {
            s.to_string()
        },
    };

    let mut curr_role = None;
    let mut line_buffer = vec![];

    for line in s.lines() {
        let trimmed = line.trim();

        // maybe a turn-separator
        if trimmed.starts_with("<|") && trimmed.ends_with("|>") {
            match trimmed.to_ascii_lowercase().as_str() {
                t @ ("user" | "system" | "assistant" | "schema") => {
                    if !line_buffer.is_empty() || curr_role.is_some() {
                        // TODO: handle previous turn
                    }

                    curr_role = Some(Role::from(t));
                    line_buffer = vec![];
                    continue;
                },
                _ => {
                    if strict_mode {
                        return _;
                    }
                },
            }
        }

        else {
            // TODO: `Role::Schema` sounds odd
            if curr_role != Some(Role::Schema) {
                line_buffer.push(parse_line(line, strict_mode)?);
            }
        }
    }
}
