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

    pub fn load(&self, name: impl AsRef<Path>) -> Result<String> {
        let path = self.root.join(name.as_ref());
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

#[cfg(test)]
mod tests {
    use super::*;

    fn vals(pairs: &[(&'static str, &str)]) -> HashMap<&'static str, String> {
        pairs.iter().map(|&(k, v)| (k, v.to_string())).collect()
    }

    #[test]
    fn substitutes_single_key() {
        let rendered = render_template("Hello, {{name}}!", &vals(&[("name", "world")]));
        assert_eq!(rendered, "Hello, world!");
    }

    #[test]
    fn substitutes_multiple_keys() {
        let rendered = render_template(
            "{{a}} + {{b}} = {{c}}",
            &vals(&[("a", "1"), ("b", "2"), ("c", "3")]),
        );
        assert_eq!(rendered, "1 + 2 = 3");
    }

    #[test]
    fn unknown_placeholder_is_left_as_is() {
        let rendered = render_template("{{known}} {{unknown}}", &vals(&[("known", "hi")]));
        assert_eq!(rendered, "hi {{unknown}}");
    }

    #[test]
    fn empty_values_map_leaves_template_unchanged() {
        let t = "No {{placeholders}} here";
        let rendered = render_template(t, &HashMap::new());
        assert_eq!(rendered, t);
    }

    #[test]
    fn repeated_placeholder_substituted_every_time() {
        let rendered = render_template("{{x}}-{{x}}-{{x}}", &vals(&[("x", "y")]));
        assert_eq!(rendered, "y-y-y");
    }

    #[test]
    fn template_store_load_returns_error_for_missing_file() {
        let store = TemplateStore::new("/nonexistent/path/that/does/not/exist");
        let result = store.load("missing.md");
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("Failed to read template"));
    }

    #[test]
    fn template_store_load_reads_file_contents() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.md");
        std::fs::write(&path, "hello from template").unwrap();
        let store = TemplateStore::new(dir.path());
        let contents = store.load("test.md").unwrap();
        assert_eq!(contents, "hello from template");
    }
}
