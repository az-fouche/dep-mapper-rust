use rustpython_parser::ast::{Stmt, Mod};
use rustpython_parser::{parse, Mode};
use anyhow::Result;

/// Represents different types of Python import statements.
#[derive(Debug, Clone)]
pub enum ImportInfo {
    Simple(String),
    From { module: String, name: String },
    FromAll(String),
}

/// Visitor for collecting imports from Python AST.
struct ImportVisitor {
    imports: Vec<ImportInfo>,
}

impl ImportVisitor {
    /// Creates a new ImportVisitor.
    fn new() -> Self {
        Self { imports: Vec::new() }
    }

    /// Visits a Python AST statement and extracts imports.
    fn visit_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Import(import_stmt) => {
                for alias in &import_stmt.names {
                    self.imports.push(ImportInfo::Simple(alias.name.to_string()));
                }
            }
            Stmt::ImportFrom(import_from_stmt) => {
                if let Some(module) = &import_from_stmt.module {
                    for alias in &import_from_stmt.names {
                        if alias.name.as_str() == "*" {
                            self.imports.push(ImportInfo::FromAll(module.to_string()));
                        } else {
                            self.imports.push(ImportInfo::From {
                                module: module.to_string(),
                                name: alias.name.to_string(),
                            });
                        }
                    }
                }
            }
            _ => {}
        }
    }

    /// Visits a collection of Python AST statements.
    fn visit_body(&mut self, body: &[Stmt]) {
        for stmt in body {
            self.visit_stmt(stmt);
        }
    }
}

/// Extracts imports from Python source code.
pub fn extract_imports(python_code: &str) -> Result<Vec<ImportInfo>> {
    let ast = parse(python_code, Mode::Module, "<string>")?;
    
    let mut visitor = ImportVisitor::new();
    match ast {
        Mod::Module(module) => visitor.visit_body(&module.body),
        Mod::Interactive(interactive) => visitor.visit_body(&interactive.body),
        Mod::Expression(_) => {}, // No statements to visit in expression mode
        Mod::FunctionType(_) => {}, // No statements to visit in function type mode
    }
    
    Ok(visitor.imports)
}