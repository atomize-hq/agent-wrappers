use super::super::ArgSnapshot;

pub(super) fn merge_inferred_args(args: &mut Vec<ArgSnapshot>, inferred: Vec<ArgSnapshot>) {
    for inf in inferred {
        if let Some(existing) = args.iter_mut().find(|a| a.name == inf.name) {
            existing.required |= inf.required;
            existing.variadic |= inf.variadic;
            if existing.note.is_none() {
                existing.note = inf.note;
            }
            continue;
        }
        args.push(inf);
    }
}

pub(super) fn infer_args_from_usage(
    usage: &str,
    cmd_path: &[String],
    has_subcommands: bool,
) -> Vec<ArgSnapshot> {
    let mut out = Vec::new();

    for line in usage.lines() {
        let tokens: Vec<&str> = line.split_whitespace().collect();
        if tokens.is_empty() {
            continue;
        }

        let mut idx = 0usize;
        if tokens.first().is_some_and(|t| *t == "claude") {
            idx += 1;
        }

        let mut matches = true;
        for p in cmd_path {
            if tokens.get(idx).is_some_and(|t| *t == p) {
                idx += 1;
            } else {
                matches = false;
                break;
            }
        }
        if !matches {
            continue;
        }

        for t in tokens.iter().skip(idx) {
            if t.starts_with("[OPTIONS]") || t.starts_with("[options]") {
                continue;
            }
            if t == &"<command>" || t == &"[command]" {
                continue;
            }
            if t == &"<prompt>" || t == &"[prompt]" {
                continue;
            }
            if t == &"<subcommands...>" || t == &"<subcommand>" || t == &"[subcommand]" {
                continue;
            }

            if has_subcommands && matches!(t.to_ascii_uppercase().as_str(), "COMMAND" | "ARGS") {
                continue;
            }

            let required = t.starts_with('<') && t.ends_with('>');
            let mut name = t.trim_start_matches('<').trim_end_matches('>').to_string();
            let mut variadic = false;
            if let Some(stripped) = name.strip_suffix("...") {
                name = stripped.to_string();
                variadic = true;
            }
            if name.is_empty() {
                continue;
            }
            if name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
            {
                out.push(ArgSnapshot {
                    name,
                    required,
                    variadic,
                    note: None,
                });
            }
        }
    }

    out
}
