use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct LibraryMetadata {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub categories: Vec<String>,
    pub functions: HashMap<String, FunctionMetadata>,
    #[serde(default)]
    pub order: Vec<String>, // discovery order
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FunctionMetadata {
    pub name: String,
    // top-level directory (first segment)
    pub category: String,
    // full relative path like "data_structures/vector"
    #[serde(default)]
    pub category_path: String,
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

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchIndex {
    pub functions: Vec<SearchableFunction>,
    pub categories: Vec<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchableFunction {
    pub name: String,
    pub category: String,
    pub tags: Vec<String>,
    pub description: String,
    pub keywords: Vec<String>,
}

pub mod parser;
// Remove the missing modules for now - we'll add them as we create them
pub mod generator;
pub mod templates;
