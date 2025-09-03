use crate::*;
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, Component};
use walkdir::WalkDir;

pub struct LibftParser {
    source_dir: String,
}

impl LibftParser {
    pub fn new(source_dir: String) -> Self {
        Self { source_dir }
    }

    fn categories_root(&self) -> std::path::PathBuf {
        let src = std::path::Path::new(&self.source_dir);
        let libft = src.join("libft");
        if libft.is_dir() { libft } else { src.to_path_buf() }
    }

    pub fn parse(&self) -> anyhow::Result<LibraryMetadata> {
        let mut functions = HashMap::new();
        let categories = self.discover_categories()?;

        println!("🔍 Scanning source directory: {}", self.source_dir);
        let mut file_count = 0;

        // Parse each source file recursively
        for entry in WalkDir::new(&self.source_dir)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_file() {
                if let Some(ext) = entry.path().extension() {
                    if ext == "c" && !entry.path().to_string_lossy().contains("main.c") {
                        file_count += 1;
                        
                        // Extract function name from basename
                        let filename = entry.path().file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("unknown");
                        
                        // Skip if already processed
                        if functions.contains_key(filename) {
                            continue;
                        }

                        if let Ok(func_meta) = self.parse_c_file(entry.path()) {
                            if let Some(meta) = func_meta {
                                println!("  📄 Parsed: {} ({}) from {}", meta.name, meta.category, entry.path().display());
                                functions.insert(meta.name.clone(), meta);
                            }
                        }
                    }
                }
            }
        }

        println!("📊 Processed {} C files, found {} functions", file_count, functions.len());

        Ok(LibraryMetadata {
            name: "libft".to_string(),
            version: "1.0.0".to_string(),
            description: "42 School C Library - Extended standard library functions".to_string(),
            author: "dlesieur".to_string(),
            categories,
            functions,
            order: Vec::new(),
        })
    }

    fn discover_categories(&self) -> anyhow::Result<Vec<String>> {
        use std::ffi::OsStr;
        let src = self.categories_root();

        // Excluded folders that are not code categories
        const EXCLUDE: &[&str] = &[
            "docs", "doc", "minilibx-linux", "target", "dist", "website", "bin",
            "obj", "build", ".git", ".github", ".idea", ".vscode"
        ];

        let mut cats = Vec::new();
        if src.is_dir() {
            for entry in fs::read_dir(src)? {
                let entry = match entry { Ok(e) => e, Err(_) => continue };
                let path = entry.path();
                if !path.is_dir() { continue; }
                let name = match path.file_name().and_then(OsStr::to_str) {
                    Some(n) => n,
                    None => continue,
                };
                if name.starts_with('.') || EXCLUDE.contains(&name) { continue; }
                if self.dir_has_code(&path) {
                    cats.push(name.to_string());
                }
            }
        }
        cats.sort();
        cats.dedup();
        Ok(cats)
    }

    fn dir_has_code(&self, dir: &std::path::Path) -> bool {
        for e in walkdir::WalkDir::new(dir)
            .min_depth(1)
            .max_depth(64)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if e.file_type().is_file() {
                if let Some(ext) = e.path().extension() {
                    if ext == "c" || ext == "h" {
                        return true;
                    }
                }
            }
        }
        false
    }

    fn extract_category_from_path(&self, path: &Path) -> String {
        let root = self.categories_root();
        if let Ok(rel) = path.strip_prefix(&root) {
            if let Some(first) = rel.components().next() {
                return first.as_os_str().to_string_lossy().to_string();
            }
        }
        "misc".to_string()
    }

    fn extract_category_path_from_path(&self, path: &Path) -> String {
        let root = self.categories_root();
        if let Ok(rel) = path.strip_prefix(&root) {
            if let Some(parent) = rel.parent() {
                let mut parts = Vec::new();
                for c in parent.components() {
                    if matches!(c, Component::Normal(_)) {
                        parts.push(c.as_os_str().to_string_lossy());
                    }
                }
                let p = parts.join("/");
                if !p.is_empty() {
                    return p;
                }
            }
        }
        // fallback to top-level category
        self.extract_category_from_path(path)
    }

    fn parse_c_file(&self, path: &Path) -> anyhow::Result<Option<FunctionMetadata>> {
        let content = fs::read_to_string(path)?;
        
        // Extract function name from filename (basename without extension)
        let filename = path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        // Determine category from path
        let category = self.extract_category_from_path(path);
        let category_path = self.extract_category_path_from_path(path);

        // Parse function prototype
        let prototype = self.extract_function_prototype(&content, filename)?;
        
        // Generate metadata
        let metadata = FunctionMetadata {
            name: filename.to_string(),
            category,
            category_path,
            tags: self.generate_tags(filename, &content),
            prototype,
            description: self.extract_description(&content),
            parameters: self.extract_parameters(&content),
            return_value: self.extract_return_value(&content),
            examples: self.generate_examples(filename),
            complexity: self.extract_complexity(&content),
            notes: self.extract_notes(&content),
            see_also: self.extract_see_also(filename),
            updated_at: None,
            author_role: None,
            related: Vec::new(),
            manual_path: None,
            manual_html: None,
        };

        Ok(Some(metadata))
    }

    fn extract_function_prototype(&self, content: &str, func_name: &str) -> anyhow::Result<String> {
        // Try multiple patterns to find function definition
        let patterns = [
            format!(r"(?m)^[^/\n]*\b{}\s*\([^{{]*\)\s*{{", regex::escape(func_name)),
            format!(r"(?m)^[^/\n]*\b{}\s*\([^;]*\);", regex::escape(func_name)),
            format!(r"(?m){}\s*\([^{{;]*", regex::escape(func_name)),
        ];
        
        for pattern in &patterns {
            if let Ok(func_regex) = Regex::new(pattern) {
                if let Some(m) = func_regex.find(content) {
                    let proto = m.as_str()
                        .trim_end_matches('{')
                        .trim_end_matches(';')
                        .trim();
                    if !proto.is_empty() && proto.len() > func_name.len() {
                        return Ok(proto.to_string());
                    }
                }
            }
        }

        // Fallback: generate from function name
        Ok(format!("/* Function: {} */", func_name))
    }

    fn generate_tags(&self, func_name: &str, content: &str) -> Vec<String> {
        let mut tags = Vec::new();

        // Enhanced tag generation based on function name patterns
        if func_name.starts_with("ft_str") { tags.push("string".to_string()); }
        if func_name.starts_with("ft_mem") { tags.push("memory".to_string()); }
        if func_name.starts_with("ft_is") { tags.push("validation".to_string()); }
        if func_name.starts_with("ft_to") { tags.push("conversion".to_string()); }
        if func_name.contains("printf") { tags.push("output".to_string()); }
        if func_name.contains("scanf") { tags.push("input".to_string()); }
        if func_name.contains("list") { tags.push("linked_list".to_string()); }
        if func_name.contains("queue") { tags.push("queue".to_string()); }
        if func_name.contains("vector") { tags.push("vector".to_string()); }
        if func_name.contains("matrix") { tags.push("matrix".to_string()); }
        if func_name.contains("sort") { tags.push("sorting".to_string()); }
        if func_name.contains("search") { tags.push("searching".to_string()); }
        if func_name.contains("map") { tags.push("data_structure".to_string()); }
        if func_name.contains("window") { tags.push("graphics".to_string()); }
        if func_name.contains("render") { tags.push("rendering".to_string()); }
        if func_name.contains("pool") || func_name.contains("arena") || func_name.contains("slab") {
            tags.push("memory_management".to_string());
        }

        // Add tags based on content analysis
        if content.contains("malloc") { tags.push("allocation".to_string()); }
        if content.contains("free") { tags.push("cleanup".to_string()); }
        if content.contains("while") || content.contains("for") { tags.push("iteration".to_string()); }
        if content.contains("recursive") || content.contains("recursion") { tags.push("recursion".to_string()); }
        if content.contains("mlx_") { tags.push("minilibx".to_string()); }
        if content.contains("pthread") { tags.push("threading".to_string()); }

        // Add difficulty tags
        if tags.contains(&"recursion".to_string()) || tags.contains(&"threading".to_string()) {
            tags.push("advanced".to_string());
        } else if tags.contains(&"allocation".to_string()) || tags.contains(&"data_structure".to_string()) {
            tags.push("intermediate".to_string());
        } else {
            tags.push("basic".to_string());
        }

        tags
    }

    fn extract_description(&self, content: &str) -> String {
        // Enhanced comment extraction with multiple patterns
        let patterns = [
            r"/\*\*\s*(.*?)\s*\*/",  // /** comment */
            r"/\*\s*(.*?)\s*\*/",    // /* comment */
            r"//\s*(.*)",            // // comment
        ];
        
        for pattern in &patterns {
            if let Ok(regex) = Regex::new(pattern) {
                if let Some(captures) = regex.captures(content) {
                    if let Some(comment) = captures.get(1) {
                        let desc = comment.as_str()
                            .lines()
                            .map(|line| line.trim().trim_start_matches('*').trim())
                            .filter(|line| !line.is_empty() && 
                                          !line.contains("****************") &&
                                          !line.contains(":::      ::::::::"))
                            .collect::<Vec<_>>()
                            .join(" ");
                        
                        if !desc.is_empty() && desc.len() > 10 {
                            return desc;
                        }
                    }
                }
            }
        }

        "No description available.".to_string()
    }

    fn extract_parameters(&self, _content: &str) -> Vec<Parameter> {
        // Simplified parameter extraction
        Vec::new()
    }

    fn extract_return_value(&self, _content: &str) -> String {
        "Return value description not available.".to_string()
    }

    fn generate_examples(&self, func_name: &str) -> Vec<Example> {
        // Generate basic example
        vec![Example {
            title: format!("Basic usage of {}", func_name),
            code: format!("// Example usage of {}\n// TODO: Add real example", func_name),
            output: None,
        }]
    }

    fn extract_complexity(&self, _content: &str) -> Option<String> {
        None
    }

    fn extract_notes(&self, _content: &str) -> Vec<String> {
        Vec::new()
    }

    fn extract_see_also(&self, _func_name: &str) -> Vec<String> {
        Vec::new()
    }
}
