use crate::*;
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

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
            parameters: self.extract_parameters(&content),
            return_value: self.extract_return_value(&content),
            examples: self.generate_examples(filename),
            complexity: self.extract_complexity(&content),
            notes: self.extract_notes(&content),
            see_also: self.extract_see_also(filename),
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
