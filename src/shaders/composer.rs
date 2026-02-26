// Copyright (C) Pavlo Hrytsenko <pashagricenko@gmail.com>
// SPDX-License-Identifier: GPL-3.0-or-later

use anyhow::{Context, Result};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// WGSL shader composer that resolves `// #import module_name` directives.
///
/// Each `.wgsl` file can declare imports at the top, and the composer
/// concatenates them in dependency order with deduplication.
pub struct ShaderComposer {
    modules: HashMap<String, String>,
}

impl ShaderComposer {
    /// Load all `.wgsl` files from a directory tree.
    pub fn from_directory(dir: &Path) -> Result<Self> {
        let mut modules = HashMap::new();
        Self::load_dir(dir, dir, &mut modules)?;
        Ok(Self { modules })
    }

    fn load_dir(base: &Path, dir: &Path, modules: &mut HashMap<String, String>) -> Result<()> {
        for entry in std::fs::read_dir(dir)
            .with_context(|| format!("Failed to read shader directory: {}", dir.display()))?
        {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                Self::load_dir(base, &path, modules)?;
            } else if path.extension().is_some_and(|ext| ext == "wgsl") {
                let module_name = Self::path_to_module_name(base, &path);
                let source = std::fs::read_to_string(&path)
                    .with_context(|| format!("Failed to read shader: {}", path.display()))?;
                modules.insert(module_name, source);
            }
        }
        Ok(())
    }

    /// `base/figures/sphere.wgsl` -> `figures::sphere`
    fn path_to_module_name(base: &Path, path: &Path) -> String {
        let relative = path.strip_prefix(base).unwrap_or(path);
        let stem = relative.with_extension("");
        stem.to_string_lossy()
            .replace(std::path::MAIN_SEPARATOR, "::")
    }

    /// Compose a shader by resolving all imports recursively.
    pub fn compose(&self, entry_module: &str) -> Result<String> {
        let mut output = String::new();
        let mut visited = HashSet::new();
        self.resolve(entry_module, &mut output, &mut visited)?;
        Ok(output)
    }

    fn resolve(
        &self,
        module_name: &str,
        output: &mut String,
        visited: &mut HashSet<String>,
    ) -> Result<()> {
        if visited.contains(module_name) {
            return Ok(());
        }
        visited.insert(module_name.to_string());

        let source = self
            .modules
            .get(module_name)
            .with_context(|| format!("Shader module not found: {module_name}"))?;

        // Resolve imports first, then emit non-import lines — single pass.
        let mut body = String::new();
        for line in source.lines() {
            let trimmed = line.trim();
            if let Some(import_name) = trimmed.strip_prefix("// #import ") {
                self.resolve(import_name.trim(), output, visited)?;
            } else {
                body.push_str(line);
                body.push('\n');
            }
        }
        output.push_str(&body);
        output.push('\n');

        Ok(())
    }

    pub fn register(&mut self, name: &str, source: &str) {
        self.modules.insert(name.to_string(), source.to_string());
    }

    /// Locate the shader directory, checking multiple locations:
    /// 1. `<exe_dir>/shaders/` — release distributions (archives, installers, AppImage)
    /// 2. `<exe_dir>/../Resources/shaders/` — macOS `.app` bundle
    /// 3. `<exe_dir>/../../src/shaders/wgsl` — `cargo run` (target/debug/ or target/release/)
    /// 4. `src/shaders/wgsl` — fallback relative to CWD
    pub fn shader_dir() -> PathBuf {
        if let Ok(exe) = std::env::current_exe() {
            if let Some(exe_dir) = exe.parent() {
                let candidates = [
                    // Release distribution: shaders/ next to the binary
                    exe_dir.join("shaders"),
                    // macOS .app bundle: Contents/MacOS/../Resources/shaders
                    exe_dir.join("../Resources/shaders"),
                    // cargo run: executable is in target/{debug,release}/
                    exe_dir.join("../../src/shaders/wgsl"),
                ];
                for path in &candidates {
                    if path.exists() {
                        return path.clone();
                    }
                }
            }
        }

        PathBuf::from("src/shaders/wgsl")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_composer(entries: &[(&str, &str)]) -> ShaderComposer {
        let mut composer = ShaderComposer {
            modules: HashMap::new(),
        };
        for &(name, src) in entries {
            composer.register(name, src);
        }
        composer
    }

    #[test]
    fn test_import_resolution() {
        let composer = make_composer(&[
            ("utils", "fn helper() -> f32 { return 1.0; }"),
            ("main", "// #import utils\nfn main() { let x = helper(); }"),
        ]);

        let result = composer.compose("main").unwrap();
        assert!(result.contains("fn helper()"));
        assert!(result.contains("fn main()"));
        assert!(result.find("fn helper()").unwrap() < result.find("fn main()").unwrap());
    }

    #[test]
    fn test_deduplication() {
        let composer = make_composer(&[
            ("base", "fn base_fn() {}"),
            ("a", "// #import base\nfn a_fn() {}"),
            ("b", "// #import base\nfn b_fn() {}"),
            ("main", "// #import a\n// #import b\nfn main_fn() {}"),
        ]);

        let result = composer.compose("main").unwrap();
        assert_eq!(result.matches("fn base_fn()").count(), 1);
    }
}
