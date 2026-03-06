use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

pub struct TemplateStore {
    root: PathBuf,
}

impl TemplateStore {
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
        }
    }

    pub fn load(&self, name: &str) -> Result<String> {
        let path = self.root.join(name);
        fs::read_to_string(&path)
            .with_context(|| format!("Failed to read template {}", path.display()))
    }
}

pub fn render_template(template: &str, values: &HashMap<&str, String>) -> String {
    let mut output = template.to_string();
    for (key, value) in values {
        let placeholder = format!("{{{{{key}}}}}");
        output = output.replace(&placeholder, value);
    }
    output
}
