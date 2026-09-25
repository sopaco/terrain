use std::path::{Path, PathBuf};

use regex::Regex;

use crate::doc::write_doc;
use crate::error::Result;
use crate::path_portable::path_in_repo;
use crate::paths::KnowledgePaths;
use crate::repo_walk::repo_file_walk;
use crate::render::{interface_body, interface_frontmatter, route_body, route_frontmatter};
use crate::schema::{InterfaceMeta, RouteMeta};

pub struct OpenApiImporter<'a> {
    paths: &'a KnowledgePaths,
    project_slug: &'a str,
}

impl<'a> OpenApiImporter<'a> {
    pub fn new(paths: &'a KnowledgePaths, project_slug: &'a str) -> Self {
        Self {
            paths,
            project_slug,
        }
    }

    pub fn import_repo(&self, repo_path: &str) -> Result<Option<usize>> {
        let repo = Path::new(repo_path);
        // Docs from a previous scan (e.g. generated from dependency specs that
        // are no longer discovered) must not survive a re-scan.
        self.clear_generated_docs()?;
        let specs = find_openapi_specs(repo);
        if specs.is_empty() {
            return Ok(None);
        }

        let mut written = 0;
        for spec_path in specs {
            written += self.import_file(repo, &spec_path)?;
        }
        Ok(Some(written))
    }

    /// Drop previously generated interface/route docs so a re-scan reflects the
    /// current spec set. These doc types are only produced by this importer.
    fn clear_generated_docs(&self) -> Result<()> {
        for sub in [
            crate::schema::DocType::Interface.subdir(),
            crate::schema::DocType::Route.subdir(),
        ]
        .into_iter()
        .flatten()
        {
            let dir = self.paths.project_dir(self.project_slug).join(sub);
            if dir.is_dir() {
                std::fs::remove_dir_all(&dir)?;
            }
        }
        Ok(())
    }

    fn import_file(&self, repo: &Path, spec_path: &Path) -> Result<usize> {
        let content = std::fs::read_to_string(spec_path)?;
        let value: serde_json::Value = if spec_path.extension().is_some_and(|e| e == "yaml" || e == "yml") {
            let yaml: serde_yaml::Value = serde_yaml::from_str(&content)?;
            serde_json::to_value(yaml)?
        } else {
            serde_json::from_str(&content)?
        };

        let paths_obj = value
            .get("paths")
            .and_then(|p| p.as_object())
            .cloned()
            .unwrap_or_default();

        let source = path_in_repo(repo, spec_path);
        let mut count = 0;

        for (path, item) in paths_obj {
            let Some(item_obj) = item.as_object() else {
                continue;
            };

            for (method, op) in item_obj {
                let method_l = method.to_lowercase();
                if !matches!(
                    method_l.as_str(),
                    "get" | "post" | "put" | "patch" | "delete" | "head" | "options"
                ) {
                    continue;
                }

                let description = op
                    .get("summary")
                    .or_else(|| op.get("description"))
                    .and_then(|v| v.as_str());

                let iface = InterfaceMeta {
                    method: method_l.clone(),
                    path: path.clone(),
                    version: value
                        .get("info")
                        .and_then(|i| i.get("version"))
                        .and_then(|v| v.as_str())
                        .map(str::to_string),
                };

                let slug = interface_slug(&method_l, &path);
                let fm = interface_frontmatter(self.project_slug, None, &iface, &source);
                let body = interface_body(&iface, description);
                write_doc(
                    self.paths
                        .doc_path(self.project_slug, crate::schema::DocType::Interface, &slug),
                    &fm,
                    &body,
                )?;
                count += 1;

                let route = RouteMeta {
                    uri: path.clone(),
                    // Specs without `operationId` (common for vendored dependency
                    // specs) fall back to the method+path pair instead of a
                    // misleading literal "unknown".
                    handler: op
                        .get("operationId")
                        .and_then(|v| v.as_str())
                        .map(str::to_string)
                        .unwrap_or_else(|| {
                            format!("{} {}", method_l.to_uppercase(), path)
                        }),
                    middleware: Vec::new(),
                };
                let route_slug = route_slug(&path, &method_l);
                let rfm = route_frontmatter(self.project_slug, &route, &source);
                let rbody = route_body(&route);
                write_doc(
                    self.paths
                        .doc_path(self.project_slug, crate::schema::DocType::Route, &route_slug),
                    &rfm,
                    &rbody,
                )?;
                count += 1;
            }
        }

        Ok(count)
    }
}

/// Discover OpenAPI specs under `repo`, honouring `.gitignore` and pruning
/// dependency directories (`.venv/`, `site-packages/`, `node_modules/`, …).
///
/// Shallower specs are returned last so that, when two specs declare the same
/// route, the project's own spec wins over a nested/vendored one.
fn find_openapi_specs(repo: &Path) -> Vec<PathBuf> {
    let name_re = Regex::new(r"(?i)openapi\.(ya?ml|json)$").unwrap();
    let mut specs = Vec::new();

    for entry in repo_file_walk(repo, None).filter_map(|e| e.ok()) {
        if !entry.file_type().is_some_and(|ft| ft.is_file()) {
            continue;
        }
        let path = entry.path();
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if name_re.is_match(file_name) || file_name.ends_with(".openapi.json") {
            specs.push(path.to_path_buf());
        }
    }

    specs.sort_by(|a, b| {
        a.components()
            .count()
            .cmp(&b.components().count())
            .then_with(|| a.cmp(b))
    });
    specs
}

fn interface_slug(method: &str, path: &str) -> String {
    let raw = format!("{method}-{path}");
    slug::slugify(raw.trim_matches('/').replace('/', "-"))
}

fn route_slug(path: &str, method: &str) -> String {
    interface_slug(method, path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn write_spec(repo: &Path, rel: &str) {
        let path = repo.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, r#"{"openapi":"3.0.0","paths":{}}"#).unwrap();
    }

    #[test]
    fn skips_gitignored_dependency_specs() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path();
        fs::write(repo.join(".gitignore"), ".venv/\n").unwrap();
        write_spec(repo, ".venv/Lib/site-packages/litellm/proxy/openapi.json");
        write_spec(repo, "api/openapi.json");

        let specs = find_openapi_specs(repo);

        assert_eq!(specs.len(), 1, "venv spec must not be indexed: {specs:?}");
        assert!(specs[0].ends_with("api/openapi.json"), "{specs:?}");
    }

    #[test]
    fn skips_vendor_dirs_without_gitignore() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path();
        write_spec(repo, "venv/lib/python3.12/site-packages/pkg/openapi.yaml");
        write_spec(repo, "vendor/third-party/openapi.json");
        write_spec(repo, "openapi.json");

        let specs = find_openapi_specs(repo);

        assert_eq!(specs.len(), 1, "vendor specs must be skipped: {specs:?}");
        assert!(specs[0].ends_with("openapi.json"), "{specs:?}");
    }

    #[test]
    fn project_spec_wins_over_nested_duplicate() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path();
        write_spec(repo, "docs/reference/openapi.json");
        write_spec(repo, "openapi.json");

        let specs = find_openapi_specs(repo);

        assert_eq!(specs.len(), 2);
        // Shallower (project-level) spec is returned last and therefore wins on slug collisions.
        assert!(specs.last().unwrap().ends_with("openapi.json"), "{specs:?}");
    }
}
