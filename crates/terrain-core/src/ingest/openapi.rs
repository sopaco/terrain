use std::path::{Path, PathBuf};

use regex::Regex;
use walkdir::WalkDir;

use crate::doc::write_doc;
use crate::error::Result;
use crate::path_portable::path_in_repo;
use crate::paths::KnowledgePaths;
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
                    handler: op
                        .get("operationId")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string(),
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

fn find_openapi_specs(repo: &Path) -> Vec<PathBuf> {
    let name_re = Regex::new(r"(?i)openapi\.(ya?ml|json)$").unwrap();
    let mut specs = Vec::new();

    for entry in WalkDir::new(repo)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path();
        if should_skip(path) {
            continue;
        }
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if name_re.is_match(file_name) || file_name.ends_with(".openapi.json") {
            specs.push(path.to_path_buf());
        }
    }

    specs.sort();
    specs
}

fn interface_slug(method: &str, path: &str) -> String {
    let raw = format!("{method}-{path}");
    slug::slugify(raw.trim_matches('/').replace('/', "-"))
}

fn route_slug(path: &str, method: &str) -> String {
    interface_slug(method, path)
}

fn should_skip(path: &Path) -> bool {
    path.components().any(|c| {
        matches!(
            c.as_os_str().to_str(),
            Some(".git" | "node_modules" | "target" | "dist")
        )
    })
}
