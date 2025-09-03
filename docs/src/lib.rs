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
