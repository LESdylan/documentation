use clap::Parser;
use std::fs;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use regex::Regex;
use walkdir::WalkDir;
use std::path::Path;

// Copy the structures locally since we can't import from lib.rs in a binary
#[derive(Debug, Serialize, Deserialize)]
pub struct LibraryMetadata {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub categories: Vec<String>,
    pub functions: HashMap<String, FunctionMetadata>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FunctionMetadata {
    pub name: String,
    pub category: String,
    pub tags: Vec<String>,
    pub prototype: String,
    pub description: String,
    pub parameters: Vec<Parameter>,
    pub return_value: String,
    pub examples: Vec<Example>,
    pub complexity: Option<String>,
    pub notes: Vec<String>,
    pub see_also: Vec<String>,
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

    pub fn parse(&self) -> anyhow::Result<LibraryMetadata> {
        let mut functions = HashMap::new();
        let categories = self.discover_categories()?;

        println!("🔍 Scanning source directory: {}", self.source_dir);
        let mut file_count = 0;

        // Parse each source file
        for entry in WalkDir::new(&self.source_dir)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_file() {
                if let Some(ext) = entry.path().extension() {
                    if ext == "c" && !entry.path().to_string_lossy().contains("main.c") {
                        file_count += 1;
                        if let Ok(func_meta) = self.parse_c_file(entry.path()) {
                            if let Some(meta) = func_meta {
                                println!("  📄 Parsed: {} ({})", meta.name, meta.category);
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
        })
    }

    fn discover_categories(&self) -> anyhow::Result<Vec<String>> {
        let mut categories = Vec::new();
        
        for entry in WalkDir::new(&self.source_dir)
            .max_depth(3)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_dir() {
                if let Some(dir_name) = entry.file_name().to_str() {
                    if !dir_name.starts_with('.') && 
                       !dir_name.starts_with("obj") && 
                       !dir_name.starts_with("build") &&
                       dir_name != "libft" &&
                       dir_name != "src" {
                        categories.push(dir_name.to_string());
                    }
                }
            }
        }

        // Add specific categories based on your libft structure
        categories.extend([
            "ctype".to_string(),
            "debug".to_string(),
            "memory".to_string(),
            "math".to_string(),
            "stdio".to_string(),
            "strings".to_string(),
            "stdlib".to_string(),
            "time".to_string(),
            "render".to_string(),
            "data_structures".to_string(),
            "sort".to_string(),
        ]);

        categories.sort();
        categories.dedup();
        Ok(categories)
    }

    fn parse_c_file(&self, path: &Path) -> anyhow::Result<Option<FunctionMetadata>> {
        let content = fs::read_to_string(path)?;
        
        // Skip files that don't contain function definitions
        if !content.contains("(") || content.contains("main.c") {
            return Ok(None);
        }

        // Extract function name from filename
        let filename = path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        // Determine category from path
        let category = self.extract_category_from_path(path);

        // Parse function prototype
        let prototype = self.extract_function_prototype(&content, filename)?;
        
        // Generate metadata
        let metadata = FunctionMetadata {
            name: filename.to_string(),
            category,
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
        };

        Ok(Some(metadata))
    }

    fn extract_category_from_path(&self, path: &Path) -> String {
        let path_str = path.to_string_lossy();
        
        if path_str.contains("strings") { "strings".to_string() }
        else if path_str.contains("memory") { "memory".to_string() }
        else if path_str.contains("math") { "math".to_string() }
        else if path_str.contains("stdio") { "stdio".to_string() }
        else if path_str.contains("data_structures") { "data_structures".to_string() }
        else if path_str.contains("render") { "render".to_string() }
        else if path_str.contains("stdlib") { "stdlib".to_string() }
        else if path_str.contains("ctype") { "ctype".to_string() }
        else { "misc".to_string() }
    }

    fn extract_function_prototype(&self, content: &str, func_name: &str) -> anyhow::Result<String> {
        // Look for function definition
        let func_regex = Regex::new(&format!(r"(?m)^[^/]*{}[^{{]*{{", regex::escape(func_name)))?;
        
        if let Some(m) = func_regex.find(content) {
            let proto = m.as_str().trim_end_matches('{').trim();
            return Ok(proto.to_string());
        }

        // Fallback: generate from function name
        Ok(format!("/* Generated prototype for {} */", func_name))
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
    
    // Write metadata JSON
    let metadata_json = serde_json::to_string_pretty(&metadata)?;
    fs::write(format!("{}/metadata.json", args.output), metadata_json)?;
    
    // Generate basic HTML page
    let html_content = generate_basic_html(&metadata)?;
    fs::write(format!("{}/index.html", args.output), html_content)?;
    
    println!("✅ Documentation generated in: {}", args.output);
    
    Ok(())
}

fn generate_basic_html(metadata: &LibraryMetadata) -> anyhow::Result<String> {
    let mut html = String::new();
    html.push_str("<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n");
    html.push_str("    <meta charset=\"UTF-8\">\n");
    html.push_str("    <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\n");
    html.push_str(&format!("    <title>{} Documentation</title>\n", metadata.name));
    html.push_str("    <style>\n");
    html.push_str(include_str!("../static/styles.css"));
    html.push_str("    </style>\n</head>\n<body>\n");
    
    // Header
    html.push_str("    <header>\n");
    html.push_str(&format!("        <h1>{} Documentation</h1>\n", metadata.name));
    html.push_str(&format!("        <p>{}</p>\n", metadata.description));
    html.push_str("    </header>\n");
    
    // Main content
    html.push_str("    <main>\n");
    html.push_str("        <section class=\"overview\">\n");
    html.push_str("            <h2>Library Overview</h2>\n");
    html.push_str(&format!("            <p>Total Functions: {}</p>\n", metadata.functions.len()));
    html.push_str(&format!("            <p>Categories: {}</p>\n", metadata.categories.len()));
    html.push_str("        </section>\n");
    
    // Categories
    html.push_str("        <section class=\"categories\">\n");
    html.push_str("            <h2>Categories</h2>\n<ul>\n");
    for category in &metadata.categories {
        let count = metadata.functions.values()
            .filter(|f| f.category == *category)
            .count();
        html.push_str(&format!("                <li>{} ({} functions)</li>\n", category, count));
    }
    html.push_str("            </ul>\n        </section>\n");
    
    // Functions list
    html.push_str("        <section class=\"functions\">\n");
    html.push_str("            <h2>Functions</h2>\n");
    
    let grouped = group_functions_by_category(&metadata.functions);
    for (category, functions) in grouped {
        html.push_str(&format!("            <h3>{}</h3>\n", category));
        html.push_str("            <div class=\"function-grid\">\n");
        
        for func in functions {
            html.push_str("                <div class=\"function-card\">\n");
            html.push_str(&format!("                    <h4>{}</h4>\n", func.name));
            html.push_str(&format!("                    <p>{}</p>\n", func.description));
            html.push_str("                    <div class=\"tags\">\n");
            for tag in &func.tags {
                html.push_str(&format!("                        <span class=\"tag\">{}</span>\n", tag));
            }
            html.push_str("                    </div>\n");
            html.push_str(&format!("                    <code>{}</code>\n", func.prototype));
            html.push_str("                </div>\n");
        }
        html.push_str("            </div>\n");
    }
    
    html.push_str("        </section>\n    </main>\n</body>\n</html>\n");
    
    Ok(html)
}

fn group_functions_by_category(functions: &HashMap<String, FunctionMetadata>) 
    -> std::collections::BTreeMap<String, Vec<&FunctionMetadata>> {
    let mut grouped = std::collections::BTreeMap::new();
    
    for func in functions.values() {
        grouped.entry(func.category.clone())
            .or_insert_with(Vec::new)
            .push(func);
    }
    
    // Sort functions within each category
    for functions in grouped.values_mut() {
        functions.sort_by(|a, b| a.name.cmp(&b.name));
    }
    
    grouped
}
