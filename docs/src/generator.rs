use clap::Parser;
use std::fs;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, BTreeSet};
use regex::Regex;
use walkdir::WalkDir;
use std::path::{Path, Component};
use std::ffi::OsStr;
use markdown::to_html; // for manual markdown -> html

// Copy the structures locally since we can't import from lib.rs in a binary
#[derive(Debug, Serialize, Deserialize)]
pub struct LibraryMetadata {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub categories: Vec<String>,
    pub functions: HashMap<String, FunctionMetadata>,
    #[serde(default)]
    pub order: Vec<String>, // discovery order (filenames/manuals)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FunctionMetadata {
    pub name: String,
    pub category: String, // top-level
    #[serde(default)]
    pub category_path: String, // nested, e.g., "data_structures/vector"
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub prototype: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub parameters: Vec<Parameter>,
    #[serde(default)]
    pub return_value: String,
    #[serde(default)]
    pub examples: Vec<Example>,
    pub complexity: Option<String>,
    #[serde(default)]
    pub notes: Vec<String>,
    #[serde(default)]
    pub see_also: Vec<String>,
    // --- SPA manual fields (optional) ---
    #[serde(default)]
    pub updated_at: Option<String>,
    #[serde(default)]
    pub author_role: Option<String>,
    #[serde(default)]
    pub related: Vec<String>,
    #[serde(default)]
    pub manual_path: Option<String>,
    #[serde(default)]
    pub manual_html: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Parameter {
    pub name: String,
    pub type_name: String,
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Example {
    pub title: String,
    pub code: String,
    pub output: Option<String>,
}

// Copy the parser locally
pub struct LibftParser {
    source_dir: String,
}

impl LibftParser {
    pub fn new(source_dir: String) -> Self {
        Self { source_dir }
    }

    fn categories_root(&self) -> std::path::PathBuf {
        let src = Path::new(&self.source_dir);
        let libft = src.join("libft");
        if libft.is_dir() { libft } else { src.to_path_buf() }
    }

    fn discover_categories(&self) -> anyhow::Result<Vec<String>> {
        let src = self.categories_root();

        const EXCLUDE: &[&str] = &[
            "docs", "doc", "minilibx-linux", "target", "dist", "website", "bin",
            "obj", "build", ".git", ".github", ".idea", ".vscode"
        ];

        let mut cats = Vec::new();
        if src.is_dir() {
            for entry in std::fs::read_dir(src)? {
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

    fn dir_has_code(&self, dir: &Path) -> bool {
        for e in WalkDir::new(dir)
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

    pub fn parse(&self) -> anyhow::Result<LibraryMetadata> {
        let mut functions = HashMap::new();
        let categories = self.discover_categories()?;
        let mut order: Vec<String> = Vec::new();

        println!("🔍 Scanning source directory: {}", self.source_dir);
        let mut file_count = 0;

        // Parse each source/header file recursively
        for entry in WalkDir::new(&self.source_dir)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if !entry.file_type().is_file() { continue; }
            let ext = match entry.path().extension().and_then(|e| e.to_str()) {
                Some(e) => e, None => continue
            };

            if ext == "c" && !entry.path().to_string_lossy().contains("main.c") {
                file_count += 1;
                
                // Extract function name from basename (without .c extension)
                let filename = entry.path().file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown");
                
                // Skip if already processed
                if functions.contains_key(filename) {
                    continue;
                }

                if let Ok(func_meta) = self.parse_c_file(entry.path()) {
                    if let Some(meta) = func_meta {
                        if !order.iter().any(|n| n == &meta.name) {
                            order.push(meta.name.clone());
                        }
                        println!("  📄 Parsed: {} ({}) from {}", meta.name, meta.category, entry.path().display());
                        functions.insert(meta.name.clone(), meta);
                    }
                }
            } else if ext == "h" {
                // Parse function prototypes from headers to ensure nodes exist
                self.parse_header_file(entry.path(), &mut functions, &mut order)?;
            }
        }

        println!("📊 Processed {} C files, found {} functions", file_count, functions.len());

        // Load manual JSON docs and merge (override C/header parsing if duplicates)
        let manuals = self.load_manuals()?;
        for (name, mut meta) in manuals {
            if meta.category_path.trim().is_empty() {
                meta.category_path = meta.category.clone();
            }
            if meta.category.trim().is_empty() {
                meta.category = meta.category_path.split('/').next().unwrap_or("misc").to_string();
            }
            if !order.iter().any(|n| n == &name) {
                order.push(name.clone());
            }
            functions.insert(name, meta);
        }

        Ok(LibraryMetadata {
            name: "libft".to_string(),
            version: "1.0.0".to_string(),
            description: "42 School C Library - Extended standard library functions".to_string(),
            author: "dlesieur".to_string(),
            categories,
            functions,
            order,
        })
    }

    // Parse function prototypes from header files; add missing nodes
    fn parse_header_file(
        &self,
        path: &Path,
        functions: &mut HashMap<String, FunctionMetadata>,
        order: &mut Vec<String>,
    ) -> anyhow::Result<()> {
        let content = fs::read_to_string(path)?;
        // Match lines like: "ret_type ft_name(args);"
        let re = Regex::new(r"(?m)^\s*[A-Za-z_][\w\s\*\(\)]*\s+(ft_[A-Za-z0-9_]+)\s*\([^;{]*\)\s*;")?;
        for cap in re.captures_iter(&content) {
            let fname = cap.get(1).unwrap().as_str().to_string();
            if !functions.contains_key(&fname) {
                let category = self.extract_category_from_path(path);
                let category_path = self.extract_category_path_from_path(path);
                let prototype_line = cap.get(0).unwrap().as_str().trim().trim_end_matches(';').to_string();
                let meta = FunctionMetadata {
                    name: fname.clone(),
                    category,
                    category_path,
                    tags: self.generate_tags(&fname, &content),
                    prototype: prototype_line,
                    description: self.extract_description(&content),
                    parameters: Vec::new(),
                    return_value: "Return value description not available.".to_string(),
                    examples: vec![Example {
                        title: format!("Basic usage of {}", fname),
                        code: format!("// Example usage of {}\n// TODO: Add real example", fname),
                        output: None,
                    }],
                    complexity: None,
                    notes: Vec::new(),
                    see_also: Vec::new(),
                    updated_at: None,
                    author_role: None,
                    related: Vec::new(),
                    manual_path: None,
                    manual_html: None,
                };
                if !order.iter().any(|n| n == &fname) {
                    order.push(fname.clone());
                }
                println!("  📄 Parsed header function: {} ({})", fname, meta.category);
                functions.insert(fname, meta);
            }
        }
        Ok(())
    }

    fn load_manuals(&self) -> anyhow::Result<HashMap<String, FunctionMetadata>> {
        let mut out = HashMap::new();
        let root = self.categories_root();

        // Scan common locations plus generic docs/ recursively
        let candidates = [
            root.join("docs").join("man"),
            root.join("docs").join("api"),
            root.join("docs"),
            Path::new(&self.source_dir).join("docs").join("man"),
            Path::new(&self.source_dir).join("docs").join("api"),
            Path::new(&self.source_dir).join("docs"),
        ];

        for base in candidates {
            if !base.is_dir() { continue; }
            for e in WalkDir::new(&base).min_depth(1).into_iter().filter_map(|e| e.ok()) {
                if e.file_type().is_file() && e.path().extension().and_then(|s| s.to_str()) == Some("json") {
                    let json_path = e.path().to_path_buf();
                    if let Ok(txt) = fs::read_to_string(&json_path) {
                        match serde_json::from_str::<FunctionMetadata>(&txt) {
                            Ok(mut meta) => {
                                // fallback name from filename
                                if meta.name.trim().is_empty() {
                                    if let Some(stem) = json_path.file_stem().and_then(|s| s.to_str()) {
                                        meta.name = stem.to_string();
                                    }
                                }
                                // if only category provided, reuse it as path
                                if meta.category_path.trim().is_empty() && !meta.category.trim().is_empty() {
                                    meta.category_path = meta.category.clone();
                                }
                                // derive top-level from category_path if missing
                                if meta.category.trim().is_empty() && !meta.category_path.trim().is_empty() {
                                    meta.category = meta.category_path.split('/').next().unwrap_or("misc").to_string();
                                }
                                // load manual markdown if present (manual_path is relative to JSON file directory)
                                if let Some(man_rel) = &meta.manual_path {
                                    let man_file = json_path.parent().unwrap_or(Path::new(".")).join(man_rel);
                                    if let Ok(md) = fs::read_to_string(&man_file) {
                                        let html = to_html(&md);
                                        meta.manual_html = Some(html);
                                    }
                                }
                                out.insert(meta.name.clone(), meta);
                            }
                            Err(err) => {
                                eprintln!("Skipping manual (invalid JSON) {}: {}", json_path.display(), err);
                            }
                        }
                    }
                }
            }
        }
        Ok(out)
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

        // Parse function prototype - look for actual function definition first
        let prototype = self.extract_function_prototype(&content, filename)?;
        
        // Generate metadata
        let metadata = FunctionMetadata {
            name: filename.to_string(),
            category,
            category_path,
            tags: self.generate_tags(filename, &content),
            prototype,
            description: self.extract_description(&content),
            parameters: Vec::new(),
            return_value: "Return value description not available.".to_string(),
            examples: vec![Example {
                title: format!("Basic usage of {}", filename),
                code: format!("// Example usage of {}\n// TODO: Add real example", filename),
                output: None,
            }],
            complexity: None,
            notes: Vec::new(),
            see_also: Vec::new(),
            updated_at: None,
            author_role: None,
            related: Vec::new(),
            manual_path: None,
            manual_html: None,
        };

        Ok(Some(metadata))
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
        self.extract_category_from_path(path)
    }

    fn extract_function_prototype(&self, content: &str, func_name: &str) -> anyhow::Result<String> {
        // Try multiple patterns to find function definition
        let patterns = [
            // Standard function definition with return type
            format!(r"(?m)^[^/\n]*\b{}\s*\([^{{]*\)\s*{{", regex::escape(func_name)),
            // Function prototype in header
            format!(r"(?m)^[^/\n]*\b{}\s*\([^;]*\);", regex::escape(func_name)),
            // Simple pattern
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

        // Add tags based on content analysis
        if content.contains("malloc") { tags.push("allocation".to_string()); }
        if content.contains("free") { tags.push("cleanup".to_string()); }
        if content.contains("while") || content.contains("for") { tags.push("iteration".to_string()); }

        // Add difficulty tags
        if tags.contains(&"allocation".to_string()) {
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
}

#[derive(Parser)]
#[command(name = "doc-generator")]
#[command(about = "Generate documentation for libft")]
struct Args {
    #[arg(short, long, default_value = ".")]
    source: String,
    
    #[arg(short, long, default_value = "dist")]
    output: String,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    
    println!("🔍 Parsing libft source code from: {}", args.source);
    
    let parser = LibftParser::new(args.source);
    let metadata = parser.parse()?;
    
    println!("📝 Found {} functions in {} categories", 
             metadata.functions.len(), 
             metadata.categories.len());
    
    // Create output directory
    fs::create_dir_all(&args.output)?;
    // Copy stylesheet to output/styles.css
    copy_stylesheet(&args.output)?;

    // Write metadata JSON
    let metadata_json = serde_json::to_string_pretty(&metadata)?;
    fs::write(format!("{}/metadata.json", args.output), metadata_json)?;
    
    // Generate basic HTML page
    let html_content = generate_basic_html(&metadata)?;
    fs::write(format!("{}/index.html", args.output), html_content)?;
    
    println!("✅ Documentation generated in: {}", args.output);
    
    Ok(())
}

// Anchor/id helpers and grouping by full path
fn sanitize_id(s: &str) -> String {
    s.to_ascii_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect()
}

fn group_functions_by_path<'a>(
	functions: &'a HashMap<String, FunctionMetadata>,
	order: &'a [String],
) -> BTreeMap<String, Vec<&'a FunctionMetadata>> {
    let mut grouped: BTreeMap<String, Vec<&FunctionMetadata>> = BTreeMap::new();

    // Build order map for stable ordering
    let order_map: HashMap<&str, usize> = order.iter().enumerate().map(|(i, n)| (n.as_str(), i)).collect();

    for f in functions.values() {
        let key = if f.category_path.trim().is_empty() { f.category.clone() } else { f.category_path.clone() };
        grouped.entry(key).or_default().push(f);
    }
    for v in grouped.values_mut() {
        v.sort_by_key(|f| {
            let pos = order_map.get(f.name.as_str()).copied().unwrap_or(usize::MAX);
            (pos, f.name.as_str())
        });
    }
    grouped
}

fn all_category_paths(grouped: &BTreeMap<String, Vec<&FunctionMetadata>>) -> BTreeSet<String> {
    let mut set = BTreeSet::new();
    for path in grouped.keys() {
        let mut acc = String::new();
        for (i, part) in path.split('/').enumerate() {
            if i == 0 { acc.push_str(part); } else { acc.push('/'); acc.push_str(part); }
            set.insert(acc.clone());
        }
    }
    set
}

fn parent_path(p: &str) -> Option<String> {
    p.rsplit_once('/').map(|(a, _)| a.to_string())
}

fn indent_level(p: &str) -> usize {
    if p.is_empty() { 0 } else { p.matches('/').count() }
}

fn sanitize_tag_class(tag: &str) -> String {
    tag.to_ascii_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect()
}

// pick a stylesheet to copy into the output dir as styles.css
fn find_css_file() -> Option<std::path::PathBuf> {
    let root = env!("CARGO_MANIFEST_DIR");
    let candidates = [
        "static/scss/main.css", // preferred: compiled SCSS
        "static/styles.css",    // legacy CSS
        "styles.css",           // repo root (fallback)
    ];
    for rel in candidates {
        let path = std::path::Path::new(root).join(rel);
        if path.is_file() {
            return Some(path);
        }
    }
    None
}

fn copy_stylesheet(output: &str) -> anyhow::Result<()> {
    let dest = std::path::Path::new(output).join("styles.css");
    if let Some(src) = find_css_file() {
        std::fs::copy(src, &dest)?;
    } else {
        // ensure the file exists to avoid 404s
        std::fs::write(&dest, "/* styles not found */")?;
    }
    Ok(())
}

fn generate_basic_html(metadata: &LibraryMetadata) -> anyhow::Result<String> {
	let mut html = String::new();
	
	// HTML document start
	html.push_str(r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>libft Documentation</title>
    <link href="https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600;700;800&family=JetBrains+Mono:wght@400;500;600&display=swap" rel="stylesheet">
    <link href="https://cdnjs.cloudflare.com/ajax/libs/font-awesome/6.0.0/css/all.min.css" rel="stylesheet">
    <link rel="stylesheet" href="styles.css">
</head>
<body>
    <header class="header">
        <div class="header__content">
            <h1 class="header__title">libft Documentation</h1>
            <p class="header__subtitle">42 School C Library - Extended standard library functions</p>
        </div>
        <div class="header__particles"></div>
    </header>

    <nav class="navigation">
        <div class="navigation__container">
            <div class="navigation__search">
                <input type="text" class="search__input" placeholder="Search functions..." id="searchInput">
                <i class="fas fa-search search__icon"></i>
            </div>
            <div class="navigation__filters">
                <span class="filter__tag active" data-filter="all">All</span>
                <span class="filter__tag" data-filter="basic">Basic</span>
                <span class="filter__tag" data-filter="intermediate">Intermediate</span>
                <span class="filter__tag" data-filter="advanced">Advanced</span>
            </div>
        </div>
    </nav>

	<div class="main-content">
		<section class="overview" id="view-home">
			<h2 class="overview__title">Library Overview</h2>
			<div class="stats-grid">
				<div class="stat-card">
					<span class="stat-number">"#);
	html.push_str(&metadata.functions.len().to_string());
	html.push_str(r#"</span>
					<span class="stat-label">Total Functions</span>
				</div>
				<div class="stat-card">
					<span class="stat-number">"#);
	html.push_str(&metadata.categories.len().to_string());
	html.push_str(r#"</span>
					<span class="stat-label">Categories</span>
				</div>
				<div class="stat-card">
					<span class="stat-number">libft</span>
					<span class="stat-label">Root Library</span>
				</div>
				<div class="stat-card">
					<span class="stat-number">✨</span>
					<span class="stat-label">Quality</span>
				</div>
			</div>
		</section>

		<section class="categories" id="view-categories">
			<h2 class="categories__title"><i class="fas fa-folder-open"></i> Library Structure</h2>
			<ul class="categories__grid">
"#);

	// Categories with function counts -> SPA route links
	for category in &metadata.categories {
		let count = metadata.functions.values().filter(|f| f.category == *category).count();
		html.push_str(&format!(
			"				<li class=\"category-item\">
					<a class=\"category-link\" href=\"#/category/{}\">
						<i class=\"fas fa-folder\"></i>
						<span class=\"category-name\">{}</span>
						<span class=\"category-count\">{} functions</span>
					</a>
				</li>
",
			category, category, count
		));
	}

	html.push_str(r#"			</ul>
		</section>

		<section class="functions-header" id="view-functions">
			<h2 class="functions__title"><i class="fas fa-code"></i> Functions Browser</h2>
			<p class="functions__subtitle">Click on any function card to see details, or use the tree navigation</p>
		</section>
"#);

	let grouped = group_functions_by_path(&metadata.functions, &metadata.order);
	// let cat_paths = all_category_paths(&grouped);
	let _cat_paths = all_category_paths(&grouped); // silence unused variable for now

	// Build tree structure for better navigation
	let tree_structure = build_tree_structure(&grouped);

	html.push_str(r#"
		<div class="layout">
			<aside class="sidebar">
				<div class="sidebar__title">
					<i class="fas fa-sitemap"></i> Library Tree
				</div>
				<div class="tree-container">
"#);

	// Generate hierarchical tree
	html.push_str(&generate_tree_html(&tree_structure, &grouped));

	html.push_str(r#"
				</div>
			</aside>
			<main>
"#);

	// Sections per path with function cards
	for (path, funcs) in grouped {
		let id = sanitize_id(&format!("cat-{}", path));
		let is_directory = path.contains('/');
		let icon = if is_directory { "fas fa-folder-open" } else { "fas fa-file-code" };
		
		html.push_str(&format!(r#"
				<section class="func-section" id="{}" data-path="{}">
					<h2><i class="{}"></i> {}</h2>
					<div class="path-breadcrumb">
						<span class="breadcrumb-item">libft</span>
"#, id, path, icon, path));

		// Add breadcrumb navigation
		let parts: Vec<&str> = path.split('/').collect();
		for (i, part) in parts.iter().enumerate() {
			let path_so_far = parts[..=i].join("/");
			html.push_str(&format!(
				"						<span class=\"breadcrumb-sep\">→</span>
						<a href=\"#/category/{}\" class=\"breadcrumb-item\">{}</a>",
				path_so_far, part
			));
		}

		html.push_str(r#"
					</div>
					<div class="function-grid">
"#);

		for func in funcs {
			let has_manual = func.manual_html.as_ref().map(|s| !s.is_empty()).unwrap_or(false);
			let complexity_icon = match func.tags.iter().find(|t| ["basic", "intermediate", "advanced"].contains(&t.as_str())) {
				Some(level) => match level.as_str() {
					"basic" => "fas fa-circle text-green",
					"intermediate" => "fas fa-adjust text-orange", 
					"advanced" => "fas fa-exclamation-triangle text-red",
					_ => "fas fa-circle text-gray"
				},
				None => "fas fa-circle text-gray"
			};

			let complexity_level = func.tags.iter()
				.find(|t| ["basic", "intermediate", "advanced"].contains(&t.as_str()))
				.map(|s| s.as_str())
				.unwrap_or("unknown");

			html.push_str(&format!(r#"						<div class="function-card" data-func="{}" data-has-manual="{}">
							<div class="function-card__header">
								<h4 class="function-card__title">
									<i class="fas fa-function"></i> {}
								</h4>
								<div class="function-card__meta">
									<i class="{}"></i>
									{}<span class="manual-indicator">{}</span>
								</div>
							</div>
							<p class="function-card__description">{}</p>
							<div class="function-card__tags">
"#, func.name, has_manual, func.name, complexity_icon, 
    complexity_level, if has_manual { "📖" } else { "" }, func.description));

			for tag in &func.tags {
				let class = sanitize_tag_class(tag);
				html.push_str(&format!(r#"								<span class="tag {}">{}</span>
"#, class, tag));
			}
			html.push_str(&format!(r#"							</div>
							<div class="function-card__code">{}</div>
							<div class="function-card__actions">
								<button class="btn-preview" onclick="showQuickPreview('{}')">
									<i class="fas fa-eye"></i> Preview
								</button>
								<button class="btn-details" onclick="showFullDocs('{}')">
									<i class="fas fa-book-open"></i> Full Docs
								</button>
							</div>
						</div>
"#, func.prototype, func.name, func.name));

			// Hidden manual template for full docs view
			if let Some(manual_html) = &func.manual_html {
				let tid = format!("manual-{}", func.name);
				html.push_str(&format!(r#"<template id="{}">{}</template>
"#, sanitize_id(&tid), manual_html));
			}
		}

		html.push_str("					</div>\n				</section>\n");
	}

	html.push_str("			</main>\n		</div>\n	</div>\n");

	// Quick Preview Modal
	html.push_str(r#"
	<div id="quick-preview-modal" class="modal-overlay hidden">
		<div class="modal-container quick-preview">
			<div class="modal-header">
				<h3 id="preview-title">Function Preview</h3>
				<button class="modal-close" onclick="closeQuickPreview()">✕</button>
			</div>
			<div class="modal-content">
				<div class="preview-prototype">
					<h4>Prototype</h4>
					<code id="preview-prototype"></code>
				</div>
				<div class="preview-description">
					<h4>Description</h4>
					<p id="preview-description"></p>
				</div>
				<div class="preview-tags">
					<h4>Tags</h4>
					<div id="preview-tags"></div>
				</div>
				<div class="preview-actions">
					<button class="btn-primary" onclick="showFullDocsFromPreview()">
						<i class="fas fa-arrow-right"></i> View Full Documentation
					</button>
				</div>
			</div>
		</div>
	</div>

	<div id="full-docs-modal" class="modal-overlay hidden">
		<div class="modal-container full-docs">
			<div class="modal-header">
				<h3 id="docs-title">Documentation</h3>
				<button class="modal-close" onclick="closeFullDocs()">✕</button>
			</div>
			<div class="modal-content" id="docs-content">
				<!-- Full documentation content -->
			</div>
		</div>
	</div>
"#);

	// Enhanced JavaScript
	html.push_str(r#"
	<script>
		let currentPreviewFunction = '';

		// Quick preview functionality
		function showQuickPreview(funcName) {
			currentPreviewFunction = funcName;
			const card = document.querySelector(`[data-func="${funcName}"]`);
			if (!card) return;

			const title = card.querySelector('.function-card__title').textContent.trim();
			const description = card.querySelector('.function-card__description').textContent;
			const prototype = card.querySelector('.function-card__code').textContent;
			const tags = Array.from(card.querySelectorAll('.tag')).map(tag => tag.outerHTML).join('');

			document.getElementById('preview-title').textContent = title;
			document.getElementById('preview-prototype').textContent = prototype;
			document.getElementById('preview-description').textContent = description;
			document.getElementById('preview-tags').innerHTML = tags;

			document.getElementById('quick-preview-modal').classList.remove('hidden');
			document.body.style.overflow = 'hidden';
		}

		function closeQuickPreview() {
			document.getElementById('quick-preview-modal').classList.add('hidden');
			document.body.style.overflow = 'auto';
		}

		function showFullDocsFromPreview() {
			closeQuickPreview();
			showFullDocs(currentPreviewFunction);
		}

		function showFullDocs(funcName) {
			// Use same sanitation as Rust sanitize_id: lower, non-alnum -> '-'
			const manualId = ('manual-' + funcName).toLowerCase().replace(/[^a-z0-9]/g, '-');
			const template = document.getElementById(manualId);
			const docsContent = document.getElementById('docs-content');
			const docsTitle = document.getElementById('docs-title');

			docsTitle.textContent = funcName;

			if (template) {
				docsContent.innerHTML = template.innerHTML;
			} else {
				// Fallback to card info
				const card = document.querySelector(`[data-func="${funcName}"]`);
				if (card) {
					const title = card.querySelector('.function-card__title').textContent.trim();
					const description = card.querySelector('.function-card__description').textContent;
					const prototype = card.querySelector('.function-card__code').textContent;
					const tags = Array.from(card.querySelectorAll('.tag')).map(tag => tag.outerHTML).join('');

					docsContent.innerHTML = `
						<h1>${title}</h1>
						<h2>Description</h2>
						<p>${description}</p>
						<h2>Prototype</h2>
						<pre><code>${prototype}</code></pre>
						<h2>Tags</h2>
						<div class="function-card__tags">${tags}</div>
						<div class="no-manual-notice">
							<i class="fas fa-info-circle"></i>
							Full manual documentation is not yet available for this function.
						</div>
					`;
				}
			}

			document.getElementById('full-docs-modal').classList.remove('hidden');
			document.body.style.overflow = 'hidden';
		}

		function closeFullDocs() {
			document.getElementById('full-docs-modal').classList.add('hidden');
			document.body.style.overflow = 'auto';
		}

		// Enhanced router with modal support
		function router() {
			const h = (location.hash || '').replace(/^#/, '');
			if (!h || h === '/' || h === '/home') {
				renderHome();
			} else if (h.startsWith('/category/')) {
				const path = h.slice('/category/'.length);
				renderCategory(path);
			} else if (h.startsWith('/function/')) {
				const name = decodeURIComponent(h.slice('/function/'.length));
				showFullDocs(name);
			} else {
				renderHome();
			}
		}

		function renderHome() {
			document.getElementById('view-home').classList.remove('hidden');
			document.getElementById('view-categories').classList.remove('hidden');
			document.getElementById('view-functions').classList.remove('hidden');
			document.querySelectorAll('.func-section').forEach(s => s.classList.remove('hidden'));
			document.querySelectorAll('.function-card').forEach(c => c.style.display = '');
		}

		function renderCategory(path) {
			document.getElementById('view-home').classList.add('hidden');
			document.getElementById('view-categories').classList.add('hidden');
			document.getElementById('view-functions').classList.remove('hidden');
			
			// Hide all sections first with transition
			document.querySelectorAll('.func-section').forEach(sec => {
				sec.style.opacity = '0';
				sec.style.transform = 'translateY(20px)';
				setTimeout(() => sec.classList.add('hidden'), 150);
			});
			
			setTimeout(() => {
				const prefix = path + '/';
				document.querySelectorAll('.func-section').forEach(sec => {
					const spath = sec.getAttribute('data-path') || '';
					if (spath === path || spath.startsWith(prefix)) {
						sec.classList.remove('hidden');
						sec.style.opacity = '1';
						sec.style.transform = 'translateY(0)';
						sec.style.transition = 'all 0.3s ease-out';
					}
				});
				
				const first = document.querySelector('.func-section:not(.hidden)');
				if (first) {
					first.scrollIntoView({ behavior: 'smooth', block: 'start' });
				}
			}, 200);
		}

		// Search functionality
		const searchInput = document.getElementById('searchInput');
		if (searchInput) {
			searchInput.addEventListener('input', function(e) {
				const query = e.target.value.toLowerCase();
				const cards = document.querySelectorAll('.function-card');
				cards.forEach(card => {
					const name = card.querySelector('.function-card__title')?.textContent?.toLowerCase() || '';
					const description = card.querySelector('.function-card__description')?.textContent?.toLowerCase() || '';
					const tags = Array.from(card.querySelectorAll('.tag')).map(t => t.textContent.toLowerCase());
					const matches = name.includes(query) || description.includes(query) || tags.some(t => t.includes(query));
					card.style.display = matches ? '' : 'none';
				});
			});
		}

		// Filter functionality
		document.querySelectorAll('.filter__tag').forEach(tag => {
			tag.addEventListener('click', function() {
				document.querySelectorAll('.filter__tag').forEach(t => t.classList.remove('active'));
				this.classList.add('active');
				const filter = this.getAttribute('data-filter');
				const cards = document.querySelectorAll('.function-card');
				cards.forEach(card => {
					if (filter === 'all') {
						card.style.display = '';
					} else {
						const tags = Array.from(card.querySelectorAll('.tag')).map(t => t.textContent.toLowerCase());
						card.style.display = tags.includes(filter) ? '' : 'none';
					}
				});
			});
		});

		// Close modals on escape key
		document.addEventListener('keydown', function(e) {
			if (e.key === 'Escape') {
				closeQuickPreview();
				closeFullDocs();
			}
		});

		// Close modals on overlay click
		document.querySelectorAll('.modal-overlay').forEach(overlay => {
			overlay.addEventListener('click', function(e) {
				if (e.target === this) {
					closeQuickPreview();
					closeFullDocs();
				}
			});
		});

		// Initialize
		window.addEventListener('hashchange', router);
		window.addEventListener('DOMContentLoaded', router);
		router();
	</script>
"#);
	Ok(html)
}

// Helper functions for tree structure
fn build_tree_structure(grouped: &BTreeMap<String, Vec<&FunctionMetadata>>) -> BTreeMap<String, TreeNode> {
	let mut tree = BTreeMap::new();
	
	for path in grouped.keys() {
		let parts: Vec<&str> = path.split('/').collect();
		let mut current_path = String::new();
		
		for (i, part) in parts.iter().enumerate() {
			if i == 0 {
				current_path = part.to_string();
			} else {
				current_path = format!("{}/{}", current_path, part);
			}
			
			tree.entry(current_path.clone()).or_insert_with(|| TreeNode {
				name: part.to_string(),
				path: current_path.clone(),
				children: Vec::new(),
				function_count: 0,
				is_leaf: i == parts.len() - 1,
			});
		}
	}
	
	// Count functions for each node
	for (path, funcs) in grouped {
		if let Some(node) = tree.get_mut(path) {
			node.function_count = funcs.len();
		}
	}
	
	tree
}

#[derive(Debug)]
struct TreeNode {
	name: String,
	path: String,
	children: Vec<String>,
	function_count: usize,
	is_leaf: bool,
}

fn generate_tree_html(tree: &BTreeMap<String, TreeNode>, grouped: &BTreeMap<String, Vec<&FunctionMetadata>>) -> String {
	let mut html = String::new();
	html.push_str("<div class=\"tree-view\">");
	
	// Get root level items (no slash in path)
	let roots: Vec<_> = tree.keys().filter(|k| !k.contains('/')).collect();
	
	for root_path in roots {
		html.push_str(&generate_tree_node_html(root_path, tree, grouped, 0));
	}
	
	html.push_str("</div>");
	html
}

fn generate_tree_node_html(path: &str, tree: &BTreeMap<String, TreeNode>, grouped: &BTreeMap<String, Vec<&FunctionMetadata>>, depth: usize) -> String {
	let node = tree.get(path).unwrap();
	let indent = "  ".repeat(depth);
	let icon = if node.is_leaf { "fas fa-file-code" } else { "fas fa-folder" };
	let count = grouped.get(path).map(|v| v.len()).unwrap_or(0);
	
	let mut html = format!(
		"{}<div class=\"tree-node\" data-depth=\"{}\">\n\
		{}  <a href=\"#/category/{}\" class=\"tree-link\">\n\
		{}    <i class=\"{}\"></i>\n\
		{}    <span class=\"tree-name\">{}</span>\n\
		{}    <span class=\"tree-count\">{}</span>\n\
		{}  </a>\n",
		indent, depth, indent, path, indent, icon, indent, node.name, indent, count, indent
	);
	
	// Add children
	let children: Vec<_> = tree.keys()
		.filter(|k| k.starts_with(&format!("{}/", path)) && k.matches('/').count() == path.matches('/').count() + 1)
		.collect();
	
	if !children.is_empty() {
		html.push_str(&format!("{}  <div class=\"tree-children\">\n", indent));
		for child_path in children {
			html.push_str(&generate_tree_node_html(child_path, tree, grouped, depth + 1));
		}
		html.push_str(&format!("{}  </div>\n", indent));
	}
	
	html.push_str(&format!("{}</div>\n", indent));
	html
}
