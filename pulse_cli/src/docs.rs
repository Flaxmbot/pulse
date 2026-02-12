//! Documentation Generator
//! 
//! Generates Markdown documentation from Pulse source files.

use std::fs;
use std::path::Path;

/// Parsed documentation item
#[derive(Debug, Clone)]
pub struct DocItem {
    pub name: String,
    pub kind: DocKind,
    pub doc: String,
    pub signature: Option<String>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum DocKind {
    Function,
    Actor,
    /// Planned for future module documentation support
    Module,
}

/// Extract documentation from Pulse source
pub fn extract_docs(source: &str) -> Vec<DocItem> {
    use pulse_compiler::Lexer;
    use pulse_compiler::Token;
    
    let mut lexer = Lexer::new(source);
    let mut items = Vec::new();
    let mut pending_doc = String::new();
    
    loop {
        match lexer.next_token() {
            Ok(Token::DocComment(doc)) => {
                if !pending_doc.is_empty() {
                    pending_doc.push('\n');
                }
                pending_doc.push_str(&doc);
            }
            Ok(Token::Fn) => {
                // Get function name
                if let Ok(Token::Identifier(name)) = lexer.next_token() {
                    items.push(DocItem {
                        name: name.clone(),
                        kind: DocKind::Function,
                        doc: std::mem::take(&mut pending_doc),
                        signature: Some(format!("fn {}(...)", name)),
                    });
                }
            }
            Ok(Token::Actor) => {
                if let Ok(Token::Identifier(name)) = lexer.next_token() {
                    items.push(DocItem {
                        name: name.clone(),
                        kind: DocKind::Actor,
                        doc: std::mem::take(&mut pending_doc),
                        signature: Some(format!("actor {}", name)),
                    });
                }
            }
            Ok(Token::Eof) => break,
            Ok(_) => {
                pending_doc.clear();
            }
            Err(_) => break,
        }
    }
    
    items
}

/// Generate Markdown documentation
pub fn generate_markdown(items: &[DocItem], module_name: &str) -> String {
    let mut md = String::new();
    
    md.push_str(&format!("# {}\n\n", module_name));
    
    // Functions
    let functions: Vec<_> = items.iter().filter(|i| matches!(i.kind, DocKind::Function)).collect();
    if !functions.is_empty() {
        md.push_str("## Functions\n\n");
        for item in functions {
            md.push_str(&format!("### `{}`\n\n", item.name));
            if let Some(sig) = &item.signature {
                md.push_str(&format!("```pulse\n{}\n```\n\n", sig));
            }
            if !item.doc.is_empty() {
                md.push_str(&format!("{}\n\n", item.doc));
            }
        }
    }
    
    // Actors
    let actors: Vec<_> = items.iter().filter(|i| matches!(i.kind, DocKind::Actor)).collect();
    if !actors.is_empty() {
        md.push_str("## Actors\n\n");
        for item in actors {
            md.push_str(&format!("### `{}`\n\n", item.name));
            if let Some(sig) = &item.signature {
                md.push_str(&format!("```pulse\n{}\n```\n\n", sig));
            }
            if !item.doc.is_empty() {
                md.push_str(&format!("{}\n\n", item.doc));
            }
        }
    }
    
    md
}

/// Generate documentation for a directory of Pulse files
pub fn generate_docs(dir: &Path, output_dir: &Path) -> Result<usize, String> {
    fs::create_dir_all(output_dir)
        .map_err(|e| format!("Failed to create output dir: {}", e))?;
    
    let files = collect_pulse_files(dir);
    let mut count = 0;
    
    for file in &files {
        let source = fs::read_to_string(file)
            .map_err(|e| format!("Failed to read {}: {}", file.display(), e))?;
        
        let items = extract_docs(&source);
        if items.is_empty() {
            continue;
        }
        
        let module_name = file.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("module");
        
        let md = generate_markdown(&items, module_name);
        
        let out_file = output_dir.join(format!("{}.md", module_name));
        fs::write(&out_file, md)
            .map_err(|e| format!("Failed to write {}: {}", out_file.display(), e))?;
        
        count += 1;
    }
    
    Ok(count)
}

fn collect_pulse_files(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && path.extension().map_or(false, |e| e == "pulse") {
                files.push(path);
            } else if path.is_dir() {
                files.extend(collect_pulse_files(&path));
            }
        }
    }
    files
}
