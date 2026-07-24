use std::fs;

use super::{pointers, FatalError, PointerRead, PointerValue, ValidateCtx};

pub(super) fn apply_fix_mode(ctx: &ValidateCtx) -> Result<(), FatalError> {
    // 1) Create missing pointer files under pointers/ for every expected target.
    for target in &ctx.expected_targets {
        for dir in ["pointers/latest_supported", "pointers/latest_validated"] {
            let path = ctx.root.join(dir).join(format!("{target}.txt"));
            if path.exists() {
                continue;
            }
            fs::create_dir_all(path.parent().unwrap_or(&ctx.root))?;
            fs::write(&path, b"none\n")?;
        }
    }

    // 2) Normalize pointer formatting (single line + trailing newline).
    for target in &ctx.expected_targets {
        for dir in ["pointers/latest_supported", "pointers/latest_validated"] {
            let path = ctx.root.join(dir).join(format!("{target}.txt"));
            pointers::normalize_single_line_file(&path)?;
        }
    }
    pointers::normalize_single_line_file(&ctx.root.join("latest_validated.txt"))?;
    pointers::normalize_single_line_file(&ctx.root.join("min_supported.txt"))?;

    // 3) Normalize current.json to match snapshots/<latest_validated>/union.json (if possible).
    let latest_validated = match pointers::read_pointer_file(
        &ctx.root.join("latest_validated.txt"),
        &ctx.stable_semver_re,
        false,
    ) {
        Ok(PointerRead::Value(PointerValue::Version(ver))) => Some(ver.to_string()),
        _ => None,
    };

    if let Some(version) = latest_validated {
        let union_path = ctx.root.join("snapshots").join(&version).join("union.json");
        if union_path.is_file() {
            let bytes = fs::read(&union_path)?;
            fs::write(ctx.root.join("current.json"), bytes)?;
        }
    }

    Ok(())
}
