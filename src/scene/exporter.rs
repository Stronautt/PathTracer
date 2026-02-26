// Copyright (C) Pavlo Hrytsenko <pashagricenko@gmail.com>
// SPDX-License-Identifier: GPL-3.0-or-later

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use super::scene::Scene;

pub fn save_scene(scene: &Scene, path: &Path) -> Result<()> {
    let yaml = serde_yml::to_string(scene).context("Failed to serialize scene")?;
    let yaml = collapse_block_arrays(&yaml);
    fs::write(path, yaml)
        .with_context(|| format!("Failed to write scene file: {}", path.display()))?;
    log::info!("Saved scene to {}", path.display());
    Ok(())
}

/// Convert block-style YAML numeric arrays to flow style:
///   key:\n  - 1.0\n  - 2.0\n  - 3.0  →  key: [1.0, 2.0, 3.0]
///
/// serde_yml puts the `- ` items at the same indent as the key line.
fn collapse_block_arrays(yaml: &str) -> String {
    let lines: Vec<&str> = yaml.lines().collect();
    let mut result = String::with_capacity(yaml.len());
    let mut i = 0;

    while i < lines.len() {
        // Look for a key line (ends with `:`) followed by `- <number>` items.
        if i + 1 < lines.len() && lines[i].ends_with(':') {
            let key_line = lines[i];
            let key_indent = key_line.len() - key_line.trim_start().len();

            // Collect consecutive `- <number>` lines at the same indent.
            let mut items = Vec::new();
            let mut j = i + 1;
            while j < lines.len() {
                let line = lines[j];
                let trimmed = line.trim_start();
                let indent = line.len() - trimmed.len();
                if indent == key_indent && trimmed.starts_with("- ") {
                    let value = trimmed[2..].trim();
                    if value.parse::<f64>().is_ok() {
                        items.push(value);
                        j += 1;
                        continue;
                    }
                }
                break;
            }

            if items.len() >= 2 {
                let key = key_line.trim_end_matches(':');
                result.push_str(key);
                result.push_str(": [");
                result.push_str(&items.join(", "));
                result.push_str("]\n");
                i = j;
                continue;
            }
        }

        result.push_str(lines[i]);
        result.push('\n');
        i += 1;
    }

    result
}
