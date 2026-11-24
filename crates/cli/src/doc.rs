//! # Syntra Documentation Generator
//!
//! Generates comprehensive API documentation for Syntra source code and standard library modules.
//! Supports both Markdown and standalone interactive static HTML websites with full-text fuzzy search,
//! responsive navigation, light/dark themes, method signatures, parameter tables, and syntax highlighting.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::exit;

use syntra_syntax::ast::*;

/// Parsed JSDoc/TSDoc comment metadata.
#[derive(Debug, Clone, Default)]
pub struct ParsedDoc {
    pub summary: String,
    pub params: Vec<(String, String)>,
    pub returns: Option<String>,
    pub throws: Vec<String>,
    pub examples: Vec<String>,
    pub since: Option<String>,
    pub author: Option<String>,
    pub see: Vec<String>,
    pub raw: String,
}

/// Documentation representation of a Syntra module (file).
#[derive(Debug, Clone)]
pub struct ModuleDoc {
    pub name: String,
    pub filename: String,
    pub summary: String,
    pub namespaces: Vec<NamespaceDoc>,
    pub classes: Vec<ClassDoc>,
    pub interfaces: Vec<InterfaceDoc>,
    pub functions: Vec<FunctionDoc>,
}

#[derive(Debug, Clone)]
pub struct NamespaceDoc {
    pub name: String,
    pub doc: Option<ParsedDoc>,
    pub functions: Vec<FunctionDoc>,
}

#[derive(Debug, Clone)]
pub struct ClassDoc {
    pub name: String,
    pub generics: Vec<String>,
    pub extends: Option<String>,
    pub implements: Vec<String>,
    pub doc: Option<ParsedDoc>,
    pub fields: Vec<FieldDoc>,
    pub constructors: Vec<ConstructorDoc>,
    pub methods: Vec<MethodDoc>,
}

#[derive(Debug, Clone)]
pub struct FieldDoc {
    pub name: String,
    pub ty: String,
    pub is_public: bool,
    pub is_static: bool,
    pub doc: Option<ParsedDoc>,
}

#[derive(Debug, Clone)]
pub struct ConstructorDoc {
    pub params: Vec<ParamDoc>,
    pub doc: Option<ParsedDoc>,
}

#[derive(Debug, Clone)]
pub struct MethodDoc {
    pub name: String,
    pub is_public: bool,
    pub is_static: bool,
    pub is_override: bool,
    pub is_async: bool,
    pub params: Vec<ParamDoc>,
    pub return_type: String,
    pub doc: Option<ParsedDoc>,
}

#[derive(Debug, Clone)]
pub struct InterfaceDoc {
    pub name: String,
    pub doc: Option<ParsedDoc>,
    pub methods: Vec<FunctionSigDoc>,
}

#[derive(Debug, Clone)]
pub struct FunctionSigDoc {
    pub name: String,
    pub is_async: bool,
    pub params: Vec<ParamDoc>,
    pub return_type: String,
    pub doc: Option<ParsedDoc>,
}

#[derive(Debug, Clone)]
pub struct FunctionDoc {
    pub name: String,
    pub is_async: bool,
    pub generics: Vec<String>,
    pub params: Vec<ParamDoc>,
    pub return_type: String,
    pub doc: Option<ParsedDoc>,
}

#[derive(Debug, Clone)]
pub struct ParamDoc {
    pub name: String,
    pub ty: String,
    pub description: Option<String>,
}

/// An indexable symbol for client-side search.
#[derive(Debug, Clone)]
pub struct SearchItem {
    pub name: String,
    pub kind: String,
    pub module: String,
    pub parent: Option<String>,
    pub url: String,
    pub description: String,
}

// =========================================================================
// HTML Escaping and Markdown Rendering Helpers
// =========================================================================

/// Escape special HTML characters.
pub fn escape_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// Convert markdown doc-comments to clean semantic HTML.
pub fn markdown_to_html(md: &str) -> String {
    let mut html = String::new();
    let lines: Vec<&str> = md.lines().collect();
    let mut in_code_block = false;
    let mut code_block_lang = String::new();
    let mut code_block_buf = String::new();
    let mut in_list = false;
    let mut para_buf = Vec::new();

    let flush_para = |p_buf: &mut Vec<String>, out: &mut String| {
        if !p_buf.is_empty() {
            let combined = p_buf.join(" ");
            let trimmed = combined.trim();
            if !trimmed.is_empty() {
                out.push_str("<p>");
                out.push_str(&inline_markdown(trimmed));
                out.push_str("</p>\n");
            }
            p_buf.clear();
        }
    };

    let flush_list = |list_flag: &mut bool, out: &mut String| {
        if *list_flag {
            out.push_str("</ul>\n");
            *list_flag = false;
        }
    };

    for line in lines {
        let trimmed = line.trim();

        if in_code_block {
            if trimmed.starts_with("```") {
                in_code_block = false;
                html.push_str("<div class=\"code-card\">\n");
                html.push_str("<div class=\"code-header\">\n");
                let lang_display = if code_block_lang.is_empty() {
                    "syntra"
                } else {
                    &code_block_lang
                };
                html.push_str(&format!(
                    "<span class=\"code-lang\">{}</span>\n",
                    escape_html(lang_display)
                ));
                html.push_str("<button class=\"copy-button\" onclick=\"copySnippet(this)\" title=\"Copy code to clipboard\">Copy</button>\n");
                html.push_str("</div>\n");
                html.push_str("<pre><code class=\"language-syntra\">");
                html.push_str(&escape_html(&code_block_buf));
                html.push_str("</code></pre>\n</div>\n");
                code_block_buf.clear();
                code_block_lang.clear();
            } else {
                if !code_block_buf.is_empty() {
                    code_block_buf.push('\n');
                }
                code_block_buf.push_str(line);
            }
            continue;
        }

        if trimmed.starts_with("```") {
            flush_para(&mut para_buf, &mut html);
            flush_list(&mut in_list, &mut html);
            in_code_block = true;
            code_block_lang = trimmed.trim_start_matches('`').trim().to_string();
            code_block_buf.clear();
            continue;
        }

        if trimmed.is_empty() {
            flush_para(&mut para_buf, &mut html);
            flush_list(&mut in_list, &mut html);
            continue;
        }

        // List item
        if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
            flush_para(&mut para_buf, &mut html);
            if !in_list {
                html.push_str("<ul class=\"doc-list\">\n");
                in_list = true;
            }
            let item_text = &trimmed[2..];
            html.push_str(&format!("<li>{}</li>\n", inline_markdown(item_text)));
            continue;
        }

        // Heading in markdown
        if let Some(stripped) = trimmed.strip_prefix("### ") {
            flush_para(&mut para_buf, &mut html);
            flush_list(&mut in_list, &mut html);
            html.push_str(&format!(
                "<h4 class=\"doc-subheading\">{}</h4>\n",
                escape_html(stripped)
            ));
            continue;
        }
        if let Some(stripped) = trimmed.strip_prefix("## ") {
            flush_para(&mut para_buf, &mut html);
            flush_list(&mut in_list, &mut html);
            html.push_str(&format!(
                "<h3 class=\"doc-heading\">{}</h3>\n",
                escape_html(stripped)
            ));
            continue;
        }

        para_buf.push(trimmed.to_string());
    }

    flush_para(&mut para_buf, &mut html);
    flush_list(&mut in_list, &mut html);

    html
}

/// Convert inline markdown (inline code, bold, links) into HTML.
fn inline_markdown(input: &str) -> String {
    let mut out = String::new();
    let chars: Vec<char> = input.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        // Inline code `code`
        if chars[i] == '`' {
            let mut j = i + 1;
            while j < len && chars[j] != '`' {
                j += 1;
            }
            if j < len {
                let code_content: String = chars[i + 1..j].iter().collect();
                out.push_str(&format!(
                    "<code class=\"inline-code\">{}</code>",
                    escape_html(&code_content)
                ));
                i = j + 1;
                continue;
            }
        }

        // Bold **text**
        if i + 1 < len && chars[i] == '*' && chars[i + 1] == '*' {
            let mut j = i + 2;
            while j + 1 < len && !(chars[j] == '*' && chars[j + 1] == '*') {
                j += 1;
            }
            if j + 1 < len {
                let bold_content: String = chars[i + 2..j].iter().collect();
                out.push_str(&format!("<strong>{}</strong>", escape_html(&bold_content)));
                i = j + 2;
                continue;
            }
        }

        // HTML escaping
        match chars[i] {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            c => out.push(c),
        }
        i += 1;
    }

    out
}

// =========================================================================
// Doc Comment Parser
// =========================================================================

/// Parse a raw JSDoc/TSDoc string into structured fields.
pub fn parse_doc_comment(raw: &str) -> ParsedDoc {
    let mut doc = ParsedDoc {
        raw: raw.to_string(),
        ..Default::default()
    };

    let mut summary_lines = Vec::new();
    let mut current_example = String::new();
    let mut in_example = false;

    for line in raw.lines() {
        let trimmed = line.trim();

        if in_example {
            if trimmed.starts_with('@') && !trimmed.starts_with("```") {
                in_example = false;
                doc.examples.push(current_example.trim().to_string());
                current_example.clear();
            } else {
                if !current_example.is_empty() {
                    current_example.push('\n');
                }
                current_example.push_str(line);
                continue;
            }
        }

        if let Some(rest) = trimmed.strip_prefix("@param ") {
            let parts: Vec<&str> = rest.splitn(2, ' ').collect();
            let name = parts.first().unwrap_or(&"").trim().to_string();
            let desc = parts.get(1).unwrap_or(&"").trim().to_string();
            doc.params.push((name, desc));
        } else if let Some(rest) = trimmed.strip_prefix("@returns ") {
            doc.returns = Some(rest.trim().to_string());
        } else if let Some(rest) = trimmed.strip_prefix("@return ") {
            doc.returns = Some(rest.trim().to_string());
        } else if let Some(rest) = trimmed.strip_prefix("@throws ") {
            doc.throws.push(rest.trim().to_string());
        } else if trimmed.starts_with("@example") {
            in_example = true;
            let example_start = trimmed.strip_prefix("@example").unwrap_or("").trim();
            if !example_start.is_empty() {
                current_example.push_str(example_start);
            }
        } else if let Some(rest) = trimmed.strip_prefix("@since ") {
            doc.since = Some(rest.trim().to_string());
        } else if let Some(rest) = trimmed.strip_prefix("@author ") {
            doc.author = Some(rest.trim().to_string());
        } else if let Some(rest) = trimmed.strip_prefix("@see ") {
            doc.see.push(rest.trim().to_string());
        } else if !trimmed.starts_with('@') {
            summary_lines.push(line);
        }
    }

    if in_example && !current_example.is_empty() {
        doc.examples.push(current_example.trim().to_string());
    }

    doc.summary = summary_lines.join("\n").trim().to_string();
    doc
}

// =========================================================================
// AST Extraction
// =========================================================================

/// Extract documentation structure from a parsed Syntra program.
pub fn extract_module_doc(
    name: &str,
    filename: &str,
    source: &str,
    program: &Program,
) -> ModuleDoc {
    let mut module_summary = String::new();

    // Check top comments of file for module summary
    let mut top_comments = Vec::new();
    for line in source.lines().take(15) {
        let trimmed = line.trim();
        if trimmed.starts_with("//") && !trimmed.starts_with("///") {
            let c = trimmed.trim_start_matches('/').trim();
            if !c.is_empty() && !c.to_lowercase().starts_with("syntra standard library") {
                top_comments.push(c);
            }
        } else if !trimmed.is_empty() && !trimmed.starts_with("//") {
            break;
        }
    }
    if !top_comments.is_empty() {
        module_summary = top_comments.join(" ");
    }

    let mut namespaces = Vec::new();
    let mut classes = Vec::new();
    let mut interfaces = Vec::new();
    let mut functions = Vec::new();

    for item in &program.items {
        match item {
            Item::Namespace(ns) => {
                let parsed_doc = ns.doc.as_ref().map(|d| parse_doc_comment(d));
                if module_summary.is_empty() {
                    if let Some(ref d) = parsed_doc {
                        module_summary = d.summary.clone();
                    }
                }
                let mut ns_funcs = Vec::new();
                for inner in &ns.items {
                    if let Item::Function(func) = inner {
                        ns_funcs.push(extract_function_doc(func));
                    }
                }
                namespaces.push(NamespaceDoc {
                    name: ns.name.clone(),
                    doc: parsed_doc,
                    functions: ns_funcs,
                });
            }
            Item::Class(cls) => {
                classes.push(extract_class_doc(cls));
            }
            Item::Interface(iface) => {
                interfaces.push(extract_interface_doc(iface));
            }
            Item::Function(func) => {
                functions.push(extract_function_doc(func));
            }
            Item::Import(_) => {}
        }
    }

    ModuleDoc {
        name: name.to_string(),
        filename: filename.to_string(),
        summary: module_summary,
        namespaces,
        classes,
        interfaces,
        functions,
    }
}

fn extract_class_doc(cls: &ClassDecl) -> ClassDoc {
    let parsed_doc = cls.doc.as_ref().map(|d| parse_doc_comment(d));

    let mut fields = Vec::new();
    let mut constructors = Vec::new();
    let mut methods = Vec::new();

    for member in &cls.members {
        match member {
            ClassMember::Field(f) => {
                let ty_str = if f.ty.is_array {
                    format!("{}[]", f.ty.name)
                } else {
                    f.ty.name.clone()
                };
                fields.push(FieldDoc {
                    name: f.name.clone(),
                    ty: ty_str,
                    is_public: f.is_public,
                    is_static: f.is_static,
                    doc: f.doc.as_ref().map(|d| parse_doc_comment(d)),
                });
            }
            ClassMember::Constructor(c) => {
                let p_docs: Vec<ParamDoc> = c
                    .params
                    .iter()
                    .map(|p| {
                        let ty_str = if p.ty.is_array {
                            format!("{}[]", p.ty.name)
                        } else {
                            p.ty.name.clone()
                        };
                        ParamDoc {
                            name: p.name.clone(),
                            ty: ty_str,
                            description: None,
                        }
                    })
                    .collect();
                constructors.push(ConstructorDoc {
                    params: p_docs,
                    doc: c.doc.as_ref().map(|d| parse_doc_comment(d)),
                });
            }
            ClassMember::Method(m) => {
                let ret = m
                    .func
                    .return_type
                    .as_ref()
                    .map(|t| {
                        if t.is_array {
                            format!("{}[]", t.name)
                        } else {
                            t.name.clone()
                        }
                    })
                    .unwrap_or_else(|| "void".to_string());

                let parsed_method_doc = m.func.doc.as_ref().map(|d| parse_doc_comment(d));

                let p_docs: Vec<ParamDoc> = m
                    .func
                    .params
                    .iter()
                    .map(|p| {
                        let ty_str = if p.ty.is_array {
                            format!("{}[]", p.ty.name)
                        } else {
                            p.ty.name.clone()
                        };
                        let desc = parsed_method_doc.as_ref().and_then(|doc| {
                            doc.params
                                .iter()
                                .find(|(name, _)| name == &p.name)
                                .map(|(_, d)| d.clone())
                        });
                        ParamDoc {
                            name: p.name.clone(),
                            ty: ty_str,
                            description: desc,
                        }
                    })
                    .collect();

                methods.push(MethodDoc {
                    name: m.func.name.clone(),
                    is_public: m.is_public,
                    is_static: m.is_static,
                    is_override: m.is_override,
                    is_async: m.func.is_async,
                    params: p_docs,
                    return_type: ret,
                    doc: parsed_method_doc,
                });
            }
        }
    }

    ClassDoc {
        name: cls.name.clone(),
        generics: cls.generic_params.clone(),
        extends: cls.extends.clone(),
        implements: cls.implements.clone(),
        doc: parsed_doc,
        fields,
        constructors,
        methods,
    }
}

fn extract_interface_doc(iface: &InterfaceDecl) -> InterfaceDoc {
    let parsed_doc = iface.doc.as_ref().map(|d| parse_doc_comment(d));

    let methods: Vec<FunctionSigDoc> = iface
        .methods
        .iter()
        .map(|m| {
            let ret = m
                .return_type
                .as_ref()
                .map(|t| {
                    if t.is_array {
                        format!("{}[]", t.name)
                    } else {
                        t.name.clone()
                    }
                })
                .unwrap_or_else(|| "void".to_string());

            let p_docs: Vec<ParamDoc> = m
                .params
                .iter()
                .map(|p| {
                    let ty_str = if p.ty.is_array {
                        format!("{}[]", p.ty.name)
                    } else {
                        p.ty.name.clone()
                    };
                    ParamDoc {
                        name: p.name.clone(),
                        ty: ty_str,
                        description: None,
                    }
                })
                .collect();

            FunctionSigDoc {
                name: m.name.clone(),
                is_async: m.is_async,
                params: p_docs,
                return_type: ret,
                doc: m.doc.as_ref().map(|d| parse_doc_comment(d)),
            }
        })
        .collect();

    InterfaceDoc {
        name: iface.name.clone(),
        doc: parsed_doc,
        methods,
    }
}

fn extract_function_doc(func: &FunctionDecl) -> FunctionDoc {
    let ret = func
        .return_type
        .as_ref()
        .map(|t| {
            if t.is_array {
                format!("{}[]", t.name)
            } else {
                t.name.clone()
            }
        })
        .unwrap_or_else(|| "void".to_string());

    let parsed_doc = func.doc.as_ref().map(|d| parse_doc_comment(d));

    let p_docs: Vec<ParamDoc> = func
        .params
        .iter()
        .map(|p| {
            let ty_str = if p.ty.is_array {
                format!("{}[]", p.ty.name)
            } else {
                p.ty.name.clone()
            };
            let desc = parsed_doc.as_ref().and_then(|doc| {
                doc.params
                    .iter()
                    .find(|(name, _)| name == &p.name)
                    .map(|(_, d)| d.clone())
            });
            ParamDoc {
                name: p.name.clone(),
                ty: ty_str,
                description: desc,
            }
        })
        .collect();

    FunctionDoc {
        name: func.name.clone(),
        is_async: func.is_async,
        generics: func.generic_params.clone(),
        params: p_docs,
        return_type: ret,
        doc: parsed_doc,
    }
}

// =========================================================================
// Main Documentation Entry Points
// =========================================================================

/// Primary documentation generator command dispatcher.
pub fn generate_docs(file: &Path, format: &str, output: Option<PathBuf>) {
    if file.is_dir() {
        generate_directory_docs(file, format, output);
    } else {
        generate_single_file_docs(file, format, output);
    }
}

/// Generates a documentation site for a directory of `.syn` files (e.g. `stdlib/`).
fn generate_directory_docs(dir: &Path, format: &str, output: Option<PathBuf>) {
    let out_dir = output.unwrap_or_else(|| PathBuf::from("docs_site"));
    if let Err(e) = fs::create_dir_all(&out_dir) {
        eprintln!(
            "❌ Failed to create output directory '{}': {}",
            out_dir.display(),
            e
        );
        exit(1);
    }

    let mut syn_files = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && path.extension().map(|e| e == "syn").unwrap_or(false) {
                syn_files.push(path);
            }
        }
    }
    syn_files.sort();

    if syn_files.is_empty() {
        eprintln!("❌ No .syn files found in directory '{}'", dir.display());
        exit(1);
    }

    println!(
        "📚 Syntra Doc Generator: Processing {} module(s) in '{}'...",
        syn_files.len(),
        dir.display()
    );

    let mut modules = Vec::new();
    for file_path in &syn_files {
        let stem = file_path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let filename = file_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let source = match fs::read_to_string(file_path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("⚠️ Warning: Skipping '{}': {}", file_path.display(), e);
                continue;
            }
        };

        match syntra_syntax::parse(&source) {
            Ok(program) => {
                let mod_doc = extract_module_doc(&stem, &filename, &source, &program);
                modules.push(mod_doc);
            }
            Err(diags) => {
                eprintln!(
                    "⚠️ Warning: Syntax errors parsing '{}' ({} diagnostics). Skipping.",
                    file_path.display(),
                    diags.len()
                );
            }
        }
    }

    if format.eq_ignore_ascii_case("html") {
        // 1. Build search index
        let search_items = build_search_index(&modules);
        let search_json = serialize_search_index(&search_items);

        // 2. Write CSS and JS
        let css_path = out_dir.join("style.css");
        let js_path = out_dir.join("search.js");
        let _ = fs::write(&css_path, get_style_css());
        let _ = fs::write(&js_path, get_search_js(&search_json));

        // 3. Write index.html
        let index_html = render_index_html(&modules);
        let index_path = out_dir.join("index.html");
        if let Err(e) = fs::write(&index_path, index_html) {
            eprintln!("❌ Error writing index.html: {e}");
            exit(1);
        }

        // 4. Write <module>.html for each module
        for m in &modules {
            let mod_html = render_module_html(m, &modules);
            let mod_path = out_dir.join(format!("{}.html", m.name));
            if let Err(e) = fs::write(&mod_path, mod_html) {
                eprintln!("❌ Error writing {}.html: {}", m.name, e);
                exit(1);
            }
        }

        println!("✨ Documentation site generated successfully!");
        println!("   📂 Location:   {}", out_dir.display());
        println!("   🏠 Home Page:  {}", index_path.display());
        println!("   📦 Modules:    {} module pages generated", modules.len());
        println!(
            "   🔍 Search:     {} searchable symbols indexed",
            search_items.len()
        );
        println!(
            "\n💡 Tip: Open '{}/index.html' in your browser to view.",
            out_dir.display()
        );
    } else {
        // Markdown format
        let mut index_md = String::new();
        index_md.push_str("# Syntra API Reference\n\n");
        index_md.push_str(&format!(
            "Comprehensive standard library documentation for **{}** modules.\n\n",
            modules.len()
        ));
        index_md.push_str("## Modules\n\n");
        index_md.push_str("| Module | Description | Classes | Functions |\n");
        index_md.push_str("| :--- | :--- | :--- | :--- |\n");

        for m in &modules {
            index_md.push_str(&format!(
                "| [`{}`]({}.md) | {} | {} | {} |\n",
                m.name,
                m.name,
                m.summary.replace('|', "\\|"),
                m.classes.len(),
                m.functions.len()
            ));

            let mut mod_md = String::new();
            format_module_markdown(&mut mod_md, m);
            let mod_path = out_dir.join(format!("{}.md", m.name));
            let _ = fs::write(&mod_path, mod_md);
        }

        let index_path = out_dir.join("README.md");
        let _ = fs::write(&index_path, index_md);
        println!(
            "✅ Generated Markdown documentation in '{}'",
            out_dir.display()
        );
    }
}

/// Generates documentation for a single `.syn` file.
fn generate_single_file_docs(file: &Path, format: &str, output: Option<PathBuf>) {
    let source = match fs::read_to_string(file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("❌ Failed to read '{}': {}", file.display(), e);
            exit(1);
        }
    };

    let program = match syntra_syntax::parse(&source) {
        Ok(p) => p,
        Err(diags) => {
            eprintln!("❌ Syntax error parsing '{}':", file.display());
            for d in diags {
                d.emit(&file.display().to_string(), &source);
            }
            exit(1);
        }
    };

    let stem = file
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let filename = file
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let mod_doc = extract_module_doc(&stem, &filename, &source, &program);

    if format.eq_ignore_ascii_case("html") {
        let html_content = render_standalone_single_file_html(&mod_doc);
        if let Some(out_path) = output {
            if let Err(e) = fs::write(&out_path, &html_content) {
                eprintln!(
                    "❌ Error writing documentation to '{}': {}",
                    out_path.display(),
                    e
                );
                exit(1);
            }
            println!(
                "✅ Generated HTML documentation at '{}'",
                out_path.display()
            );
        } else {
            print!("{html_content}");
        }
    } else {
        let mut md = String::new();
        format_module_markdown(&mut md, &mod_doc);
        if let Some(out_path) = output {
            if let Err(e) = fs::write(&out_path, &md) {
                eprintln!(
                    "❌ Error writing documentation to '{}': {}",
                    out_path.display(),
                    e
                );
                exit(1);
            }
            println!(
                "✅ Generated Markdown documentation at '{}'",
                out_path.display()
            );
        } else {
            print!("{md}");
        }
    }
}

// =========================================================================
// Markdown Formatter
// =========================================================================

fn format_module_markdown(doc_md: &mut String, m: &ModuleDoc) {
    doc_md.push_str(&format!("# Module: `{}`\n\n", m.name));
    if !m.summary.is_empty() {
        doc_md.push_str(&format!("{}\n\n", m.summary));
    }

    if !m.namespaces.is_empty() {
        doc_md.push_str("## Namespaces\n\n");
        for ns in &m.namespaces {
            doc_md.push_str(&format!("### `namespace {}`\n\n", ns.name));
            if let Some(d) = &ns.doc {
                doc_md.push_str(&format!("{}\n\n", d.summary));
            }
            for f in &ns.functions {
                format_function_markdown(doc_md, f, 4);
            }
        }
    }

    if !m.classes.is_empty() {
        doc_md.push_str("## Classes\n\n");
        for cls in &m.classes {
            let gen = if cls.generics.is_empty() {
                "".to_string()
            } else {
                format!("<{}>", cls.generics.join(", "))
            };
            let ext = cls
                .extends
                .as_ref()
                .map(|e| format!(" extends {e}"))
                .unwrap_or_default();
            doc_md.push_str(&format!("### `class {}{}{}`\n\n", cls.name, gen, ext));

            if let Some(d) = &cls.doc {
                doc_md.push_str(&format!("{}\n\n", d.summary));
                for ex in &d.examples {
                    doc_md.push_str(&format!("```syntra\n{ex}\n```\n\n"));
                }
            }

            if !cls.fields.is_empty() {
                doc_md.push_str("#### Fields\n\n");
                doc_md.push_str("| Field | Type | Access | Description |\n");
                doc_md.push_str("| :--- | :--- | :--- | :--- |\n");
                for f in &cls.fields {
                    let acc = if f.is_public { "public" } else { "private" };
                    let desc = f.doc.as_ref().map(|d| d.summary.as_str()).unwrap_or("-");
                    doc_md.push_str(&format!(
                        "| `{}` | `{}` | {} | {} |\n",
                        f.name, f.ty, acc, desc
                    ));
                }
                doc_md.push('\n');
            }

            if !cls.methods.is_empty() {
                doc_md.push_str("#### Methods\n\n");
                for meth in &cls.methods {
                    let acc = if meth.is_public { "public " } else { "" };
                    let st = if meth.is_static { "static " } else { "" };
                    let asy = if meth.is_async { "async " } else { "" };
                    let params: Vec<_> = meth
                        .params
                        .iter()
                        .map(|p| format!("{} {}", p.ty, p.name))
                        .collect();
                    doc_md.push_str(&format!(
                        "##### `{}{}{}func {}({}): {}`\n\n",
                        acc,
                        st,
                        asy,
                        meth.name,
                        params.join(", "),
                        meth.return_type
                    ));
                    if let Some(d) = &meth.doc {
                        doc_md.push_str(&format!("{}\n\n", d.summary));
                    }
                }
            }
        }
    }

    if !m.interfaces.is_empty() {
        doc_md.push_str("## Interfaces\n\n");
        for iface in &m.interfaces {
            doc_md.push_str(&format!("### `interface {}`\n\n", iface.name));
            if let Some(d) = &iface.doc {
                doc_md.push_str(&format!("{}\n\n", d.summary));
            }
            for meth in &iface.methods {
                let params: Vec<_> = meth
                    .params
                    .iter()
                    .map(|p| format!("{} {}", p.ty, p.name))
                    .collect();
                doc_md.push_str(&format!(
                    "#### `func {}({}): {}`\n\n",
                    meth.name,
                    params.join(", "),
                    meth.return_type
                ));
                if let Some(d) = &meth.doc {
                    doc_md.push_str(&format!("{}\n\n", d.summary));
                }
            }
        }
    }

    if !m.functions.is_empty() {
        doc_md.push_str("## Functions\n\n");
        for func in &m.functions {
            format_function_markdown(doc_md, func, 3);
        }
    }
}

fn format_function_markdown(doc_md: &mut String, func: &FunctionDoc, heading_level: usize) {
    let hashes = "#".repeat(heading_level);
    let asy = if func.is_async { "async " } else { "" };
    let params: Vec<_> = func
        .params
        .iter()
        .map(|p| format!("{} {}", p.ty, p.name))
        .collect();
    doc_md.push_str(&format!(
        "{} `{}func {}({}): {}`\n\n",
        hashes,
        asy,
        func.name,
        params.join(", "),
        func.return_type
    ));
    if let Some(d) = &func.doc {
        doc_md.push_str(&format!("{}\n\n", d.summary));
        if !d.params.is_empty() {
            doc_md.push_str("- **Parameters:**\n");
            for (pname, pdesc) in &d.params {
                doc_md.push_str(&format!("  - `{pname}`: {pdesc}\n"));
            }
            doc_md.push('\n');
        }
        if let Some(ret) = &d.returns {
            doc_md.push_str(&format!(
                "- **Returns:** `{}` - {}\n\n",
                func.return_type, ret
            ));
        }
    }
}

// =========================================================================
// Search Index
// =========================================================================

fn build_search_index(modules: &[ModuleDoc]) -> Vec<SearchItem> {
    let mut items = Vec::new();

    for m in modules {
        // Module
        items.push(SearchItem {
            name: m.name.clone(),
            kind: "module".to_string(),
            module: m.name.clone(),
            parent: None,
            url: format!("{}.html", m.name),
            description: m.summary.chars().take(120).collect(),
        });

        // Namespaces
        for ns in &m.namespaces {
            items.push(SearchItem {
                name: ns.name.clone(),
                kind: "namespace".to_string(),
                module: m.name.clone(),
                parent: None,
                url: format!("{}.html#ns-{}", m.name, ns.name),
                description: ns
                    .doc
                    .as_ref()
                    .map(|d| d.summary.chars().take(100).collect())
                    .unwrap_or_default(),
            });
            for f in &ns.functions {
                items.push(SearchItem {
                    name: f.name.clone(),
                    kind: "function".to_string(),
                    module: m.name.clone(),
                    parent: Some(ns.name.clone()),
                    url: format!("{}.html#func-{}-{}", m.name, ns.name, f.name),
                    description: f
                        .doc
                        .as_ref()
                        .map(|d| d.summary.chars().take(100).collect())
                        .unwrap_or_default(),
                });
            }
        }

        // Classes
        for cls in &m.classes {
            items.push(SearchItem {
                name: cls.name.clone(),
                kind: "class".to_string(),
                module: m.name.clone(),
                parent: None,
                url: format!("{}.html#class-{}", m.name, cls.name),
                description: cls
                    .doc
                    .as_ref()
                    .map(|d| d.summary.chars().take(120).collect())
                    .unwrap_or_default(),
            });

            // Methods
            for meth in &cls.methods {
                items.push(SearchItem {
                    name: meth.name.clone(),
                    kind: "method".to_string(),
                    module: m.name.clone(),
                    parent: Some(cls.name.clone()),
                    url: format!("{}.html#method-{}-{}", m.name, cls.name, meth.name),
                    description: meth
                        .doc
                        .as_ref()
                        .map(|d| d.summary.chars().take(100).collect())
                        .unwrap_or_default(),
                });
            }
        }

        // Interfaces
        for iface in &m.interfaces {
            items.push(SearchItem {
                name: iface.name.clone(),
                kind: "interface".to_string(),
                module: m.name.clone(),
                parent: None,
                url: format!("{}.html#interface-{}", m.name, iface.name),
                description: iface
                    .doc
                    .as_ref()
                    .map(|d| d.summary.chars().take(100).collect())
                    .unwrap_or_default(),
            });
        }

        // Top-level functions
        for f in &m.functions {
            items.push(SearchItem {
                name: f.name.clone(),
                kind: "function".to_string(),
                module: m.name.clone(),
                parent: None,
                url: format!("{}.html#func-{}", m.name, f.name),
                description: f
                    .doc
                    .as_ref()
                    .map(|d| d.summary.chars().take(100).collect())
                    .unwrap_or_default(),
            });
        }
    }

    items
}

fn serialize_search_index(items: &[SearchItem]) -> String {
    let mut json = String::from("[\n");
    for (i, item) in items.iter().enumerate() {
        let parent_str = match &item.parent {
            Some(p) => format!("\"{}\"", escape_json(p)),
            None => "null".to_string(),
        };
        json.push_str(&format!(
            "  {{\"name\":\"{}\",\"kind\":\"{}\",\"module\":\"{}\",\"parent\":{},\"url\":\"{}\",\"desc\":\"{}\"}}{}\n",
            escape_json(&item.name),
            escape_json(&item.kind),
            escape_json(&item.module),
            parent_str,
            escape_json(&item.url),
            escape_json(&item.description),
            if i + 1 < items.len() { "," } else { "" }
        ));
    }
    json.push(']');
    json
}

fn escape_json(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', " ")
        .replace('\r', "")
        .replace('\t', " ")
}

// =========================================================================
// HTML Rendering: Portal Landing Page (index.html)
// =========================================================================

pub fn render_index_html(modules: &[ModuleDoc]) -> String {
    let mut total_classes = 0;
    let mut total_methods = 0;
    let mut total_funcs = 0;

    for m in modules {
        total_classes += m.classes.len();
        for c in &m.classes {
            total_methods += c.methods.len();
        }
        total_funcs += m.functions.len();
        for ns in &m.namespaces {
            total_funcs += ns.functions.len();
        }
    }

    let mut out = String::new();
    out.push_str("<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n");
    out.push_str("<meta charset=\"utf-8\">\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\n");
    out.push_str("<title>Syntra Standard Library Documentation</title>\n");
    out.push_str("<link rel=\"stylesheet\" href=\"style.css\">\n");
    out.push_str("</head>\n<body>\n");

    // Navigation Header
    out.push_str(&render_header(None));

    // Main Container
    out.push_str("<main class=\"home-container\">\n");

    // Hero Section
    out.push_str("<section class=\"hero-section\">\n");
    out.push_str("<div class=\"hero-badge\">✨ Syntra v0.1.0 • 2026 Edition</div>\n");
    out.push_str("<h1 class=\"hero-title\">Syntra Standard Library</h1>\n");
    out.push_str("<p class=\"hero-subtitle\">Zero-overhead, type-safe, cross-platform standard library with pure native performance, high-throughput asynchronous I/O, and cryptographic integrity.</p>\n");

    // Stats Bar
    out.push_str("<div class=\"stats-grid\">\n");
    out.push_str(&format!("<div class=\"stat-card\"><span class=\"stat-num\">{}</span><span class=\"stat-label\">Modules</span></div>\n", modules.len()));
    out.push_str(&format!("<div class=\"stat-card\"><span class=\"stat-num\">{total_classes}</span><span class=\"stat-label\">Classes</span></div>\n"));
    out.push_str(&format!("<div class=\"stat-card\"><span class=\"stat-num\">{total_methods}</span><span class=\"stat-label\">Methods</span></div>\n"));
    out.push_str(&format!("<div class=\"stat-card\"><span class=\"stat-num\">{total_funcs}</span><span class=\"stat-label\">Functions</span></div>\n"));
    out.push_str("<div class=\"stat-card\"><span class=\"stat-num\">100%</span><span class=\"stat-label\">Documented</span></div>\n");
    out.push_str("</div>\n");
    out.push_str("</section>\n");

    // Module Grid
    out.push_str("<section class=\"modules-section\">\n");
    out.push_str("<div class=\"section-header\">\n");
    out.push_str("<h2>Standard Library Modules</h2>\n");
    out.push_str("<p>Click any module below to inspect its classes, interfaces, methods, and complete API references.</p>\n");
    out.push_str("</div>\n");

    out.push_str("<div class=\"module-grid\">\n");
    for m in modules {
        let (icon, category) = get_module_meta(&m.name);
        out.push_str(&format!(
            "<a href=\"{}.html\" class=\"module-card\">\n",
            m.name
        ));
        out.push_str("<div class=\"module-card-header\">\n");
        out.push_str(&format!("<span class=\"module-icon\">{icon}</span>\n"));
        out.push_str("<div class=\"module-info\">\n");
        out.push_str(&format!("<h3 class=\"module-name\">{}</h3>\n", m.name));
        out.push_str(&format!(
            "<span class=\"module-category\">{category}</span>\n"
        ));
        out.push_str("</div>\n");
        out.push_str("</div>\n");

        let summary = if m.summary.is_empty() {
            "Syntra standard library module component."
        } else {
            &m.summary
        };
        out.push_str(&format!(
            "<p class=\"module-desc\">{}</p>\n",
            escape_html(summary)
        ));

        out.push_str("<div class=\"module-card-footer\">\n");
        out.push_str(&format!(
            "<span class=\"badge pill\">{} classes</span>\n",
            m.classes.len()
        ));
        out.push_str(&format!(
            "<span class=\"badge pill\">{} methods</span>\n",
            m.classes.iter().map(|c| c.methods.len()).sum::<usize>()
        ));
        out.push_str("<span class=\"view-arrow\">&rarr;</span>\n");
        out.push_str("</div>\n");
        out.push_str("</a>\n");
    }
    out.push_str("</div>\n");
    out.push_str("</section>\n");

    out.push_str("</main>\n");

    // Footer
    out.push_str(&render_footer());
    out.push_str("<script src=\"search.js\"></script>\n");
    out.push_str("</body>\n</html>\n");

    out
}

// =========================================================================
// HTML Rendering: Module Page (<module>.html)
// =========================================================================

pub fn render_module_html(m: &ModuleDoc, all_modules: &[ModuleDoc]) -> String {
    let mut out = String::new();
    out.push_str("<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n");
    out.push_str("<meta charset=\"utf-8\">\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\n");
    out.push_str(&format!(
        "<title>Module: {} - Syntra Documentation</title>\n",
        m.name
    ));
    out.push_str("<link rel=\"stylesheet\" href=\"style.css\">\n");
    out.push_str("</head>\n<body>\n");

    // Header
    out.push_str(&render_header(Some(&m.name)));

    out.push_str("<div class=\"layout-wrapper\">\n");

    // Sidebar
    out.push_str("<aside class=\"sidebar\" id=\"sidebar\">\n");
    out.push_str("<div class=\"sidebar-section\">\n");
    out.push_str("<h4 class=\"sidebar-title\">Standard Library</h4>\n");
    out.push_str("<ul class=\"sidebar-nav\">\n");
    for other in all_modules {
        let is_active = other.name == m.name;
        let active_class = if is_active { " class=\"active\"" } else { "" };
        out.push_str(&format!(
            "<li><a href=\"{}.html\"{}>{}</a></li>\n",
            other.name, active_class, other.name
        ));
    }
    out.push_str("</ul>\n");
    out.push_str("</div>\n");

    // In-page TOC
    out.push_str("<div class=\"sidebar-section toc-section\">\n");
    out.push_str("<h4 class=\"sidebar-title\">On This Page</h4>\n");
    out.push_str("<ul class=\"sidebar-toc\">\n");
    if !m.namespaces.is_empty() {
        out.push_str("<li><span class=\"toc-group\">Namespaces</span><ul>\n");
        for ns in &m.namespaces {
            out.push_str(&format!(
                "<li><a href=\"#ns-{}\">{}</a></li>\n",
                ns.name, ns.name
            ));
        }
        out.push_str("</ul></li>\n");
    }
    if !m.classes.is_empty() {
        out.push_str("<li><span class=\"toc-group\">Classes</span><ul>\n");
        for cls in &m.classes {
            out.push_str(&format!(
                "<li><a href=\"#class-{}\">{}</a></li>\n",
                cls.name, cls.name
            ));
        }
        out.push_str("</ul></li>\n");
    }
    if !m.interfaces.is_empty() {
        out.push_str("<li><span class=\"toc-group\">Interfaces</span><ul>\n");
        for iface in &m.interfaces {
            out.push_str(&format!(
                "<li><a href=\"#interface-{}\">{}</a></li>\n",
                iface.name, iface.name
            ));
        }
        out.push_str("</ul></li>\n");
    }
    if !m.functions.is_empty() {
        out.push_str("<li><span class=\"toc-group\">Functions</span><ul>\n");
        for f in &m.functions {
            out.push_str(&format!(
                "<li><a href=\"#func-{}\">{}</a></li>\n",
                f.name, f.name
            ));
        }
        out.push_str("</ul></li>\n");
    }
    out.push_str("</ul>\n");
    out.push_str("</div>\n");
    out.push_str("</aside>\n");

    // Main Content
    out.push_str("<main class=\"content-main\">\n");

    // Breadcrumbs
    out.push_str("<nav class=\"breadcrumbs\">\n");
    out.push_str("<a href=\"index.html\">Home</a> &gt; <a href=\"index.html#modules\">Standard Library</a> &gt; <span class=\"current\">");
    out.push_str(&escape_html(&m.name));
    out.push_str("</span>\n</nav>\n");

    // Module Header
    let (icon, category) = get_module_meta(&m.name);
    out.push_str("<header class=\"page-header\">\n");
    out.push_str("<div class=\"page-title-row\">\n");
    out.push_str(&format!("<span class=\"header-icon\">{icon}</span>\n"));
    out.push_str(&format!("<h1>{}</h1>\n", escape_html(&m.name)));
    out.push_str(&format!(
        "<span class=\"badge category\">{category}</span>\n"
    ));
    out.push_str("</div>\n");

    if !m.summary.is_empty() {
        out.push_str(&format!(
            "<div class=\"module-lead\">{}</div>\n",
            markdown_to_html(&m.summary)
        ));
    }
    out.push_str("</header>\n");

    // Namespaces
    for ns in &m.namespaces {
        out.push_str(&format!(
            "<section id=\"ns-{}\" class=\"api-section\">\n",
            ns.name
        ));
        out.push_str("<div class=\"section-title-bar\">\n");
        out.push_str("<span class=\"kind-tag namespace\">namespace</span>\n");
        out.push_str(&format!("<h2>{}</h2>\n", escape_html(&ns.name)));
        out.push_str("</div>\n");

        if let Some(d) = &ns.doc {
            out.push_str(&format!(
                "<div class=\"doc-text\">{}</div>\n",
                markdown_to_html(&d.summary)
            ));
        }

        for f in &ns.functions {
            out.push_str(&render_function_card(f, Some(&ns.name)));
        }
        out.push_str("</section>\n");
    }

    // Classes
    for cls in &m.classes {
        out.push_str(&format!(
            "<section id=\"class-{}\" class=\"api-section card-section\">\n",
            cls.name
        ));
        out.push_str("<div class=\"section-title-bar\">\n");
        out.push_str("<span class=\"kind-tag class\">class</span>\n");
        out.push_str(&format!("<h2>{}</h2>\n", escape_html(&cls.name)));
        if let Some(ext) = &cls.extends {
            out.push_str(&format!(
                "<span class=\"inherits\">extends <code>{}</code></span>\n",
                escape_html(ext)
            ));
        }
        if !cls.implements.is_empty() {
            out.push_str(&format!(
                "<span class=\"implements\">implements <code>{}</code></span>\n",
                escape_html(&cls.implements.join(", "))
            ));
        }
        out.push_str("</div>\n");

        // Class Doc
        if let Some(d) = &cls.doc {
            if let Some(since) = &d.since {
                out.push_str(&format!("<div class=\"since-badge\"><span class=\"badge pill since\">Since {}</span></div>\n", escape_html(since)));
            }
            out.push_str(&format!(
                "<div class=\"doc-text\">{}</div>\n",
                markdown_to_html(&d.summary)
            ));

            for ex in &d.examples {
                out.push_str("<div class=\"example-box\">\n");
                out.push_str("<h4 class=\"example-title\">Example</h4>\n");
                out.push_str(&markdown_to_html(ex));
                out.push_str("</div>\n");
            }
        }

        // Fields table
        if !cls.fields.is_empty() {
            out.push_str("<div class=\"members-group\">\n");
            out.push_str("<h3 class=\"members-heading\">Fields</h3>\n");
            out.push_str("<div class=\"table-responsive\">\n");
            out.push_str("<table class=\"api-table\">\n");
            out.push_str("<thead><tr><th>Field</th><th>Type</th><th>Access</th><th>Description</th></tr></thead>\n");
            out.push_str("<tbody>\n");
            for f in &cls.fields {
                let acc = if f.is_public { "public" } else { "private" };
                let acc_class = if f.is_public {
                    "badge pub"
                } else {
                    "badge priv"
                };
                let desc = f.doc.as_ref().map(|d| d.summary.as_str()).unwrap_or("-");
                out.push_str("<tr>\n");
                out.push_str(&format!("<td><code>{}</code></td>\n", escape_html(&f.name)));
                out.push_str(&format!(
                    "<td><span class=\"type-pill\">{}</span></td>\n",
                    escape_html(&f.ty)
                ));
                out.push_str(&format!(
                    "<td><span class=\"{acc_class}\">{acc}</span></td>\n"
                ));
                out.push_str(&format!("<td>{}</td>\n", inline_markdown(desc)));
                out.push_str("</tr>\n");
            }
            out.push_str("</tbody>\n</table>\n</div>\n</div>\n");
        }

        // Methods
        if !cls.methods.is_empty() {
            out.push_str("<div class=\"members-group\">\n");
            out.push_str("<h3 class=\"members-heading\">Methods</h3>\n");
            for meth in &cls.methods {
                out.push_str(&render_method_card(meth, &cls.name));
            }
            out.push_str("</div>\n");
        }

        out.push_str("</section>\n");
    }

    // Interfaces
    for iface in &m.interfaces {
        out.push_str(&format!(
            "<section id=\"interface-{}\" class=\"api-section card-section\">\n",
            iface.name
        ));
        out.push_str("<div class=\"section-title-bar\">\n");
        out.push_str("<span class=\"kind-tag interface\">interface</span>\n");
        out.push_str(&format!("<h2>{}</h2>\n", escape_html(&iface.name)));
        out.push_str("</div>\n");

        if let Some(d) = &iface.doc {
            out.push_str(&format!(
                "<div class=\"doc-text\">{}</div>\n",
                markdown_to_html(&d.summary)
            ));
        }

        if !iface.methods.is_empty() {
            out.push_str("<div class=\"members-group\">\n");
            out.push_str("<h3 class=\"members-heading\">Method Signatures</h3>\n");
            for m_sig in &iface.methods {
                out.push_str("<div class=\"method-card\">\n");
                let params: Vec<_> = m_sig
                    .params
                    .iter()
                    .map(|p| {
                        format!(
                            "<span class=\"param-type\">{}</span> {}",
                            escape_html(&p.ty),
                            escape_html(&p.name)
                        )
                    })
                    .collect();
                out.push_str("<div class=\"method-signature\">\n");
                out.push_str(&format!("<span class=\"keyword\">func</span> <span class=\"method-name\">{}</span>({}) -&gt; <span class=\"return-type\">{}</span>\n",
                    escape_html(&m_sig.name), params.join(", "), escape_html(&m_sig.return_type)));
                out.push_str("</div>\n");
                if let Some(d) = &m_sig.doc {
                    out.push_str(&format!(
                        "<div class=\"doc-text\">{}</div>\n",
                        markdown_to_html(&d.summary)
                    ));
                }
                out.push_str("</div>\n");
            }
            out.push_str("</div>\n");
        }
        out.push_str("</section>\n");
    }

    // Standalone Functions
    if !m.functions.is_empty() {
        out.push_str("<section class=\"api-section\">\n");
        out.push_str("<div class=\"section-title-bar\">\n");
        out.push_str("<span class=\"kind-tag function\">functions</span>\n");
        out.push_str("<h2>Module Functions</h2>\n");
        out.push_str("</div>\n");
        for f in &m.functions {
            out.push_str(&render_function_card(f, None));
        }
        out.push_str("</section>\n");
    }

    out.push_str("</main>\n");
    out.push_str("</div>\n"); // layout-wrapper

    out.push_str(&render_footer());
    out.push_str("<script src=\"search.js\"></script>\n");
    out.push_str("</body>\n</html>\n");

    out
}

// =========================================================================
// HTML Rendering: Standalone Single File HTML
// =========================================================================

pub fn render_standalone_single_file_html(m: &ModuleDoc) -> String {
    let mut out = String::new();
    out.push_str("<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n");
    out.push_str("<meta charset=\"utf-8\">\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\n");
    out.push_str(&format!(
        "<title>API Reference: {} - Syntra</title>\n",
        m.name
    ));
    out.push_str("<style>\n");
    out.push_str(get_style_css());
    out.push_str("\n</style>\n</head>\n<body>\n");

    out.push_str(&render_header(Some(&m.name)));

    out.push_str("<div class=\"layout-wrapper single-col\">\n");
    out.push_str("<main class=\"content-main\">\n");

    out.push_str("<header class=\"page-header\">\n");
    out.push_str("<div class=\"page-title-row\">\n");
    out.push_str(&format!("<h1>Module: {}</h1>\n", escape_html(&m.name)));
    out.push_str("</div>\n");
    if !m.summary.is_empty() {
        out.push_str(&format!(
            "<div class=\"module-lead\">{}</div>\n",
            markdown_to_html(&m.summary)
        ));
    }
    out.push_str("</header>\n");

    // Classes
    for cls in &m.classes {
        out.push_str(&format!(
            "<section id=\"class-{}\" class=\"api-section card-section\">\n",
            cls.name
        ));
        out.push_str("<div class=\"section-title-bar\">\n");
        out.push_str("<span class=\"kind-tag class\">class</span>\n");
        out.push_str(&format!("<h2>{}</h2>\n", escape_html(&cls.name)));
        out.push_str("</div>\n");

        if let Some(d) = &cls.doc {
            out.push_str(&format!(
                "<div class=\"doc-text\">{}</div>\n",
                markdown_to_html(&d.summary)
            ));
        }

        if !cls.fields.is_empty() {
            out.push_str("<div class=\"members-group\">\n");
            out.push_str("<h3 class=\"members-heading\">Fields</h3>\n");
            out.push_str("<div class=\"table-responsive\"><table class=\"api-table\">\n");
            out.push_str("<thead><tr><th>Field</th><th>Type</th><th>Access</th><th>Description</th></tr></thead>\n<tbody>\n");
            for f in &cls.fields {
                let acc = if f.is_public { "public" } else { "private" };
                let desc = f.doc.as_ref().map(|d| d.summary.as_str()).unwrap_or("-");
                out.push_str(&format!("<tr><td><code>{}</code></td><td><span class=\"type-pill\">{}</span></td><td>{}</td><td>{}</td></tr>\n",
                    escape_html(&f.name), escape_html(&f.ty), acc, inline_markdown(desc)));
            }
            out.push_str("</tbody></table></div></div>\n");
        }

        if !cls.methods.is_empty() {
            out.push_str("<div class=\"members-group\">\n");
            out.push_str("<h3 class=\"members-heading\">Methods</h3>\n");
            for meth in &cls.methods {
                out.push_str(&render_method_card(meth, &cls.name));
            }
            out.push_str("</div>\n");
        }
        out.push_str("</section>\n");
    }

    out.push_str("</main>\n</div>\n");
    out.push_str(&render_footer());
    out.push_str("</body>\n</html>\n");

    out
}

// =========================================================================
// Component Renderers: Method Cards, Function Cards, Header, Footer
// =========================================================================

fn render_method_card(meth: &MethodDoc, class_name: &str) -> String {
    let mut out = String::new();
    let anchor_id = format!("method-{}-{}", class_name, meth.name);
    out.push_str(&format!("<div id=\"{anchor_id}\" class=\"method-card\">\n"));

    // Header bar
    out.push_str("<div class=\"method-card-header\">\n");
    out.push_str("<div class=\"method-badges\">\n");
    if meth.is_public {
        out.push_str("<span class=\"badge pub\">public</span>\n");
    }
    if meth.is_static {
        out.push_str("<span class=\"badge static\">static</span>\n");
    }
    if meth.is_async {
        out.push_str("<span class=\"badge async\">async</span>\n");
    }
    if meth.is_override {
        out.push_str("<span class=\"badge override\">override</span>\n");
    }
    if let Some(doc) = &meth.doc {
        if let Some(since) = &doc.since {
            out.push_str(&format!(
                "<span class=\"badge since\">Since {}</span>\n",
                escape_html(since)
            ));
        }
    }
    out.push_str("</div>\n"); // method-badges

    out.push_str(&format!(
        "<a href=\"#{anchor_id}\" class=\"anchor-link\" title=\"Permanent link\">#</a>\n"
    ));
    out.push_str("</div>\n"); // method-card-header

    // Signature
    let params: Vec<_> = meth
        .params
        .iter()
        .map(|p| {
            format!(
                "<span class=\"param-type\">{}</span> <span class=\"param-name\">{}</span>",
                escape_html(&p.ty),
                escape_html(&p.name)
            )
        })
        .collect();

    out.push_str("<div class=\"method-signature\">\n");
    out.push_str(&format!("<span class=\"keyword\">func</span> <span class=\"method-name\">{}</span>({}) -&gt; <span class=\"return-type\">{}</span>\n",
        escape_html(&meth.name), params.join(", "), escape_html(&meth.return_type)));
    out.push_str("</div>\n");

    // Description
    if let Some(doc) = &meth.doc {
        if !doc.summary.is_empty() {
            out.push_str(&format!(
                "<div class=\"doc-text\">{}</div>\n",
                markdown_to_html(&doc.summary)
            ));
        }

        // Parameters table
        if !meth.params.is_empty() {
            out.push_str("<div class=\"params-section\">\n");
            out.push_str("<h5 class=\"sub-title\">Parameters</h5>\n");
            out.push_str("<div class=\"table-responsive\"><table class=\"params-table\">\n");
            out.push_str(
                "<thead><tr><th>Name</th><th>Type</th><th>Description</th></tr></thead>\n<tbody>\n",
            );
            for p in &meth.params {
                let desc = p.description.as_deref().unwrap_or("-");
                out.push_str("<tr>\n");
                out.push_str(&format!("<td><code>{}</code></td>\n", escape_html(&p.name)));
                out.push_str(&format!(
                    "<td><span class=\"type-pill\">{}</span></td>\n",
                    escape_html(&p.ty)
                ));
                out.push_str(&format!("<td>{}</td>\n", inline_markdown(desc)));
                out.push_str("</tr>\n");
            }
            out.push_str("</tbody></table></div>\n</div>\n");
        }

        // Returns
        if let Some(ret_desc) = &doc.returns {
            out.push_str("<div class=\"returns-section\">\n");
            out.push_str(&format!("<h5 class=\"sub-title\">Returns</h5><p><span class=\"type-pill\">{}</span> &mdash; {}</p>\n",
                escape_html(&meth.return_type), inline_markdown(ret_desc)));
            out.push_str("</div>\n");
        }

        // Throws
        if !doc.throws.is_empty() {
            out.push_str("<div class=\"throws-section\">\n");
            out.push_str("<h5 class=\"sub-title\">Throws</h5><ul>\n");
            for th in &doc.throws {
                out.push_str(&format!("<li>{}</li>\n", inline_markdown(th)));
            }
            out.push_str("</ul></div>\n");
        }

        // Examples
        for ex in &doc.examples {
            out.push_str("<div class=\"example-box\">\n");
            out.push_str("<h5 class=\"sub-title\">Example</h5>\n");
            out.push_str(&markdown_to_html(ex));
            out.push_str("</div>\n");
        }
    }

    out.push_str("</div>\n"); // method-card
    out
}

fn render_function_card(func: &FunctionDoc, ns_name: Option<&str>) -> String {
    let mut out = String::new();
    let anchor_id = match ns_name {
        Some(ns) => format!("func-{}-{}", ns, func.name),
        None => format!("func-{}", func.name),
    };

    out.push_str(&format!("<div id=\"{anchor_id}\" class=\"method-card\">\n"));
    out.push_str("<div class=\"method-card-header\">\n");
    out.push_str("<div class=\"method-badges\">\n");
    if func.is_async {
        out.push_str("<span class=\"badge async\">async</span>\n");
    }
    if let Some(doc) = &func.doc {
        if let Some(since) = &doc.since {
            out.push_str(&format!(
                "<span class=\"badge since\">Since {}</span>\n",
                escape_html(since)
            ));
        }
    }
    out.push_str("</div>\n");
    out.push_str(&format!(
        "<a href=\"#{anchor_id}\" class=\"anchor-link\" title=\"Permanent link\">#</a>\n"
    ));
    out.push_str("</div>\n");

    let params: Vec<_> = func
        .params
        .iter()
        .map(|p| {
            format!(
                "<span class=\"param-type\">{}</span> <span class=\"param-name\">{}</span>",
                escape_html(&p.ty),
                escape_html(&p.name)
            )
        })
        .collect();

    let gen = if func.generics.is_empty() {
        "".to_string()
    } else {
        format!("&lt;{}&gt;", escape_html(&func.generics.join(", ")))
    };

    out.push_str("<div class=\"method-signature\">\n");
    out.push_str(&format!("<span class=\"keyword\">func</span> <span class=\"method-name\">{}{}</span>({}) -&gt; <span class=\"return-type\">{}</span>\n",
        escape_html(&func.name), gen, params.join(", "), escape_html(&func.return_type)));
    out.push_str("</div>\n");

    if let Some(doc) = &func.doc {
        if !doc.summary.is_empty() {
            out.push_str(&format!(
                "<div class=\"doc-text\">{}</div>\n",
                markdown_to_html(&doc.summary)
            ));
        }

        if !func.params.is_empty() {
            out.push_str("<div class=\"params-section\">\n");
            out.push_str("<h5 class=\"sub-title\">Parameters</h5>\n");
            out.push_str("<div class=\"table-responsive\"><table class=\"params-table\">\n");
            out.push_str(
                "<thead><tr><th>Name</th><th>Type</th><th>Description</th></tr></thead>\n<tbody>\n",
            );
            for p in &func.params {
                let desc = p.description.as_deref().unwrap_or("-");
                out.push_str("<tr>\n");
                out.push_str(&format!("<td><code>{}</code></td>\n", escape_html(&p.name)));
                out.push_str(&format!(
                    "<td><span class=\"type-pill\">{}</span></td>\n",
                    escape_html(&p.ty)
                ));
                out.push_str(&format!("<td>{}</td>\n", inline_markdown(desc)));
                out.push_str("</tr>\n");
            }
            out.push_str("</tbody></table></div>\n</div>\n");
        }

        if let Some(ret_desc) = &doc.returns {
            out.push_str("<div class=\"returns-section\">\n");
            out.push_str(&format!("<h5 class=\"sub-title\">Returns</h5><p><span class=\"type-pill\">{}</span> &mdash; {}</p>\n",
                escape_html(&func.return_type), inline_markdown(ret_desc)));
            out.push_str("</div>\n");
        }

        for ex in &doc.examples {
            out.push_str("<div class=\"example-box\">\n");
            out.push_str("<h5 class=\"sub-title\">Example</h5>\n");
            out.push_str(&markdown_to_html(ex));
            out.push_str("</div>\n");
        }
    }

    out.push_str("</div>\n");
    out
}

fn render_header(current_module: Option<&str>) -> String {
    let mut out = String::new();
    out.push_str("<header class=\"top-nav\">\n");
    out.push_str("<div class=\"nav-left\">\n");
    out.push_str("<button class=\"mobile-toggle\" id=\"mobileToggle\" aria-label=\"Toggle Navigation\">☰</button>\n");
    out.push_str("<a href=\"index.html\" class=\"brand-link\">\n");
    out.push_str("<span class=\"brand-logo\">🔮</span>\n");
    out.push_str("<span class=\"brand-name\">Syntra</span>\n");
    out.push_str("<span class=\"brand-badge\">Docs</span>\n");
    out.push_str("</a>\n");
    out.push_str("</div>\n");

    out.push_str("<div class=\"nav-center\">\n");
    out.push_str("<div class=\"search-container\">\n");
    out.push_str("<span class=\"search-icon\">🔍</span>\n");
    let placeholder = match current_module {
        Some(m) => format!("Search in {m} or all symbols... (/)"),
        None => "Search modules, classes, methods... (/)".to_string(),
    };
    out.push_str(&format!(
        "<input type=\"text\" id=\"searchInput\" placeholder=\"{}\" autocomplete=\"off\">\n",
        escape_html(&placeholder)
    ));
    out.push_str("<span class=\"search-shortcut\">/</span>\n");
    out.push_str("<div id=\"searchResults\" class=\"search-dropdown hidden\"></div>\n");
    out.push_str("</div>\n");
    out.push_str("</div>\n");

    out.push_str("<div class=\"nav-right\">\n");
    out.push_str("<a href=\"index.html\" class=\"nav-link\">Modules</a>\n");
    out.push_str("<button id=\"themeToggle\" class=\"theme-toggle\" title=\"Toggle Dark / Light theme\">🌙</button>\n");
    out.push_str("</div>\n");
    out.push_str("</header>\n");
    out
}

fn render_footer() -> String {
    let mut out = String::new();
    out.push_str("<footer class=\"site-footer\">\n");
    out.push_str("<div class=\"footer-inner\">\n");
    out.push_str("<p>Built with ❤️ by the Syntra Language Team • Version 0.1.0 • Spec 2026</p>\n");
    out.push_str("<p class=\"footer-sub\">High-performance Whole-Program Adaptive Programming Platform</p>\n");
    out.push_str("</div>\n");
    out.push_str("</footer>\n");
    out
}

fn get_module_meta(name: &str) -> (&'static str, &'static str) {
    match name {
        "web" => ("🌐", "Web Framework"),
        "email" => ("✉️", "Email & MIME Engine"),
        "crypto" => ("🔒", "Cryptography"),
        "concurrency" => ("⚡", "Async & Threading"),
        "http" => ("📡", "HTTP Client/Server"),
        "net" => ("🔌", "Networking & Sockets"),
        "io" => ("💾", "I/O Streams & Buffers"),
        "file" => ("📁", "Filesystem Primitives"),
        "system" => ("⚙️", "OS Primitives & Environment"),
        "json" => ("📋", "JSON Serialization"),
        "map" => ("🗺️", "Collections & Hash Maps"),
        "loader" => ("📦", "Dynamic Module Loader"),
        "syntax" => ("🌲", "AST & Lexer Engine"),
        "checker" => ("🔍", "Type Checking & Semantics"),
        "air" => ("🧬", "Intermediate Representation"),
        "codegen" => ("⚙️", "Cranelift Code Generation"),
        _ => ("📦", "Standard Library"),
    }
}

// =========================================================================
// Assets: Embedded CSS and JavaScript
// =========================================================================

pub fn get_style_css() -> &'static str {
    r#"/* Syntra Documentation Theme: Modern, Responsive, Adaptive Dark/Light Mode */
:root {
    --font-sans: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
    --font-mono: ui-monospace, SFMono-Regular, "SF Mono", Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace;
    
    --bg-body: #f8fafc;
    --bg-surface: #ffffff;
    --bg-sidebar: #f1f5f9;
    --bg-card: #ffffff;
    --border-subtle: #e2e8f0;
    --border-card: #cbd5e1;
    --text-main: #0f172a;
    --text-muted: #64748b;
    --text-accent: #4f46e5;
    --accent-hover: #4338ca;
    
    --code-bg: #0f172a;
    --code-text: #f8fafc;
    --inline-code-bg: #f1f5f9;
    --inline-code-border: #e2e8f0;
    
    --badge-pub: #dcfce7;
    --badge-pub-text: #15803d;
    --badge-priv: #f1f5f9;
    --badge-priv-text: #64748b;
    --badge-static: #e0f2fe;
    --badge-static-text: #0369a1;
    --badge-async: #f3e8ff;
    --badge-async-text: #7e22ce;
    --badge-override: #ffedd5;
    --badge-override-text: #c2410c;
    --badge-since: #fef3c7;
    --badge-since-text: #b45309;
    
    --shadow-sm: 0 1px 2px 0 rgb(0 0 0 / 0.05);
    --shadow-card: 0 4px 6px -1px rgb(0 0 0 / 0.08), 0 2px 4px -2px rgb(0 0 0 / 0.04);
}

[data-theme="dark"] {
    --bg-body: #0b0f19;
    --bg-surface: #111827;
    --bg-sidebar: #0f172a;
    --bg-card: #1e293b;
    --border-subtle: #1e293b;
    --border-card: #334155;
    --text-main: #f8fafc;
    --text-muted: #94a3b8;
    --text-accent: #818cf8;
    --accent-hover: #a5b4fc;
    
    --code-bg: #090d16;
    --code-text: #e2e8f0;
    --inline-code-bg: #1e293b;
    --inline-code-border: #334155;
    
    --badge-pub: #064e3b;
    --badge-pub-text: #86efac;
    --badge-priv: #1e293b;
    --badge-priv-text: #94a3b8;
    --badge-static: #0c4a6e;
    --badge-static-text: #7dd3fc;
    --badge-async: #581c87;
    --badge-async-text: #d8b4fe;
    --badge-override: #7c2d12;
    --badge-override-text: #fdba74;
    --badge-since: #78350f;
    --badge-since-text: #fde68a;
    
    --shadow-card: 0 4px 6px -1px rgb(0 0 0 / 0.4);
}

* { box-sizing: border-box; margin: 0; padding: 0; }

body {
    font-family: var(--font-sans);
    background-color: var(--bg-body);
    color: var(--text-main);
    line-height: 1.6;
    -webkit-font-smoothing: antialiased;
}

a { color: var(--text-accent); text-decoration: none; transition: color 0.15s; }
a:hover { color: var(--accent-hover); text-decoration: underline; }

/* Top Navigation Bar */
.top-nav {
    position: sticky;
    top: 0;
    z-index: 100;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.75rem 1.5rem;
    background-color: var(--bg-surface);
    border-bottom: 1px solid var(--border-subtle);
    box-shadow: var(--shadow-sm);
}

.nav-left { display: flex; align-items: center; gap: 1rem; }
.brand-link { display: flex; align-items: center; gap: 0.5rem; font-weight: 700; font-size: 1.25rem; color: var(--text-main); }
.brand-link:hover { text-decoration: none; }
.brand-logo { font-size: 1.4rem; }
.brand-badge { font-size: 0.75rem; background: var(--text-accent); color: #fff; padding: 0.15rem 0.45rem; border-radius: 9999px; font-weight: 600; text-transform: uppercase; }

.nav-center { flex: 1; max-width: 500px; margin: 0 1.5rem; }
.search-container { position: relative; display: flex; align-items: center; }
.search-container input {
    width: 100%;
    padding: 0.5rem 2rem 0.5rem 2.25rem;
    font-size: 0.875rem;
    background-color: var(--bg-body);
    border: 1px solid var(--border-subtle);
    border-radius: 8px;
    color: var(--text-main);
    outline: none;
    transition: all 0.2s;
}
.search-container input:focus {
    border-color: var(--text-accent);
    box-shadow: 0 0 0 3px rgba(99, 102, 241, 0.2);
}
.search-icon { position: absolute; left: 0.75rem; font-size: 0.875rem; pointer-events: none; opacity: 0.6; }
.search-shortcut { position: absolute; right: 0.75rem; font-size: 0.75rem; padding: 0.1rem 0.35rem; background: var(--border-subtle); border-radius: 4px; color: var(--text-muted); }

.search-dropdown {
    position: absolute;
    top: calc(100% + 0.5rem);
    left: 0;
    right: 0;
    max-height: 400px;
    overflow-y: auto;
    background: var(--bg-surface);
    border: 1px solid var(--border-card);
    border-radius: 8px;
    box-shadow: var(--shadow-card);
    z-index: 1000;
}
.search-dropdown.hidden { display: none; }
.search-item {
    display: flex;
    flex-direction: column;
    padding: 0.65rem 1rem;
    border-bottom: 1px solid var(--border-subtle);
    cursor: pointer;
    text-decoration: none !important;
}
.search-item:hover, .search-item.selected { background-color: var(--bg-sidebar); }
.search-item-header { display: flex; align-items: center; justify-content: space-between; margin-bottom: 0.2rem; }
.search-item-title { font-weight: 600; font-size: 0.9rem; color: var(--text-main); }
.search-item-meta { font-size: 0.75rem; color: var(--text-muted); }
.search-item-desc { font-size: 0.8rem; color: var(--text-muted); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }

.nav-right { display: flex; align-items: center; gap: 1rem; }
.nav-link { font-weight: 500; font-size: 0.9rem; color: var(--text-main); }
.theme-toggle {
    background: none;
    border: 1px solid var(--border-subtle);
    border-radius: 8px;
    padding: 0.4rem 0.65rem;
    cursor: pointer;
    font-size: 1rem;
    color: var(--text-main);
    transition: background 0.15s;
}
.theme-toggle:hover { background-color: var(--bg-sidebar); }

.mobile-toggle { display: none; background: none; border: none; font-size: 1.25rem; color: var(--text-main); cursor: pointer; }

/* Layout Grid */
.layout-wrapper { display: flex; max-width: 1440px; margin: 0 auto; min-height: calc(100vh - 65px); }
.layout-wrapper.single-col { max-width: 960px; padding: 2rem 1.5rem; }

/* Sidebar */
.sidebar {
    width: 280px;
    flex-shrink: 0;
    background-color: var(--bg-sidebar);
    border-right: 1px solid var(--border-subtle);
    padding: 1.5rem 1rem;
    position: sticky;
    top: 65px;
    height: calc(100vh - 65px);
    overflow-y: auto;
}
.sidebar-section { margin-bottom: 2rem; }
.sidebar-title { font-size: 0.75rem; text-transform: uppercase; letter-spacing: 0.05em; color: var(--text-muted); margin-bottom: 0.75rem; font-weight: 700; }
.sidebar-nav, .sidebar-toc { list-style: none; }
.sidebar-nav li a {
    display: block;
    padding: 0.4rem 0.75rem;
    border-radius: 6px;
    font-size: 0.9rem;
    color: var(--text-main);
    font-weight: 500;
}
.sidebar-nav li a:hover { background-color: var(--bg-surface); text-decoration: none; }
.sidebar-nav li a.active { background-color: var(--text-accent); color: #fff; font-weight: 600; }

.sidebar-toc .toc-group { font-size: 0.8rem; font-weight: 600; color: var(--text-muted); display: block; margin-top: 0.6rem; }
.sidebar-toc ul { list-style: none; padding-left: 0.75rem; margin-top: 0.25rem; }
.sidebar-toc ul li a { font-size: 0.85rem; color: var(--text-main); display: block; padding: 0.2rem 0; }
.sidebar-toc ul li a:hover { color: var(--text-accent); text-decoration: none; }

/* Content Main */
.content-main { flex: 1; min-width: 0; padding: 2rem 2.5rem; }

/* Breadcrumbs */
.breadcrumbs { font-size: 0.85rem; color: var(--text-muted); margin-bottom: 1.5rem; }
.breadcrumbs a { color: var(--text-muted); }
.breadcrumbs .current { color: var(--text-main); font-weight: 600; }

/* Page Header */
.page-header { margin-bottom: 2.5rem; border-bottom: 1px solid var(--border-subtle); padding-bottom: 1.5rem; }
.page-title-row { display: flex; align-items: center; gap: 0.75rem; margin-bottom: 0.75rem; flex-wrap: wrap; }
.page-title-row h1 { font-size: 2.25rem; font-weight: 800; }
.header-icon { font-size: 2.25rem; }
.module-lead { font-size: 1.15rem; color: var(--text-muted); margin-top: 0.5rem; }

/* API Section */
.api-section { margin-bottom: 3rem; }
.section-title-bar { display: flex; align-items: center; gap: 0.75rem; margin-bottom: 1rem; border-bottom: 1px solid var(--border-subtle); padding-bottom: 0.5rem; }
.section-title-bar h2 { font-size: 1.6rem; font-weight: 700; }

.kind-tag {
    font-size: 0.7rem;
    font-weight: 700;
    text-transform: uppercase;
    padding: 0.2rem 0.5rem;
    border-radius: 4px;
    letter-spacing: 0.05em;
}
.kind-tag.namespace { background: #dbeafe; color: #1d4ed8; }
.kind-tag.class { background: #dcfce7; color: #15803d; }
.kind-tag.interface { background: #fef3c7; color: #b45309; }
.kind-tag.function { background: #f3e8ff; color: #7e22ce; }

.inherits, .implements { font-size: 0.9rem; color: var(--text-muted); }

/* Method and Function Cards */
.method-card {
    background-color: var(--bg-card);
    border: 1px solid var(--border-card);
    border-radius: 10px;
    padding: 1.25rem 1.5rem;
    margin-bottom: 1.5rem;
    box-shadow: var(--shadow-sm);
    transition: border-color 0.15s, box-shadow 0.15s;
}
.method-card:hover { border-color: var(--text-accent); box-shadow: var(--shadow-card); }

.method-card-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 0.75rem; }
.method-badges { display: flex; gap: 0.4rem; flex-wrap: wrap; }
.anchor-link { color: var(--border-card); font-weight: bold; font-size: 1.1rem; padding: 0 0.25rem; }
.anchor-link:hover { color: var(--text-accent); text-decoration: none; }

.method-signature {
    font-family: var(--font-mono);
    font-size: 0.95rem;
    background-color: var(--bg-body);
    border: 1px solid var(--border-subtle);
    padding: 0.75rem 1rem;
    border-radius: 8px;
    margin-bottom: 1rem;
    overflow-x: auto;
}
.method-signature .keyword { color: #f43f5e; font-weight: 600; }
.method-signature .method-name { color: var(--text-accent); font-weight: 700; }
.method-signature .param-type { color: #0284c7; }
.method-signature .param-name { color: var(--text-main); }
.method-signature .return-type { color: #059669; font-weight: 600; }

.doc-text { margin-bottom: 1rem; font-size: 0.95rem; }
.doc-text p { margin-bottom: 0.75rem; }
.doc-list { margin: 0.5rem 0 1rem 1.5rem; }
.doc-list li { margin-bottom: 0.35rem; }

.sub-title { font-size: 0.85rem; font-weight: 700; text-transform: uppercase; letter-spacing: 0.05em; color: var(--text-muted); margin: 1rem 0 0.5rem 0; }

/* Badges */
.badge { display: inline-flex; align-items: center; font-size: 0.75rem; font-weight: 600; padding: 0.15rem 0.5rem; border-radius: 4px; }
.badge.pill { border-radius: 9999px; }
.badge.pub { background: var(--badge-pub); color: var(--badge-pub-text); }
.badge.priv { background: var(--badge-priv); color: var(--badge-priv-text); }
.badge.static { background: var(--badge-static); color: var(--badge-static-text); }
.badge.async { background: var(--badge-async); color: var(--badge-async-text); }
.badge.override { background: var(--badge-override); color: var(--badge-override-text); }
.badge.since { background: var(--badge-since); color: var(--badge-since-text); }
.badge.category { background: var(--bg-sidebar); color: var(--text-accent); border: 1px solid var(--border-subtle); }

.type-pill {
    font-family: var(--font-mono);
    font-size: 0.8rem;
    background-color: var(--inline-code-bg);
    border: 1px solid var(--inline-code-border);
    padding: 0.1rem 0.35rem;
    border-radius: 4px;
    color: var(--text-accent);
}

.inline-code {
    font-family: var(--font-mono);
    font-size: 0.85em;
    background-color: var(--inline-code-bg);
    border: 1px solid var(--inline-code-border);
    padding: 0.15em 0.35em;
    border-radius: 4px;
}

/* Tables */
.table-responsive { overflow-x: auto; margin-bottom: 1rem; }
.api-table, .params-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.9rem;
    text-align: left;
}
.api-table th, .params-table th {
    background-color: var(--bg-sidebar);
    padding: 0.6rem 0.85rem;
    border: 1px solid var(--border-subtle);
    font-weight: 600;
}
.api-table td, .params-table td {
    padding: 0.6rem 0.85rem;
    border: 1px solid var(--border-subtle);
    vertical-align: top;
}

/* Code Blocks */
.code-card {
    background-color: var(--code-bg);
    border-radius: 8px;
    margin: 1rem 0;
    overflow: hidden;
}
.code-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.4rem 1rem;
    background-color: rgba(255, 255, 255, 0.05);
    border-bottom: 1px solid rgba(255, 255, 255, 0.1);
}
.code-lang { font-family: var(--font-mono); font-size: 0.75rem; color: #94a3b8; text-transform: uppercase; font-weight: 600; }
.copy-button {
    background: rgba(255, 255, 255, 0.1);
    border: none;
    border-radius: 4px;
    color: #e2e8f0;
    font-size: 0.75rem;
    padding: 0.2rem 0.5rem;
    cursor: pointer;
    transition: background 0.15s;
}
.copy-button:hover { background: rgba(255, 255, 255, 0.2); }
pre { margin: 0; padding: 1rem; overflow-x: auto; }
pre code { font-family: var(--font-mono); font-size: 0.875rem; color: var(--code-text); line-height: 1.5; }

/* Home Page Specifics */
.home-container { max-width: 1200px; margin: 0 auto; padding: 3rem 1.5rem; }
.hero-section { text-align: center; margin-bottom: 3.5rem; }
.hero-badge { display: inline-block; padding: 0.25rem 0.75rem; background: var(--bg-sidebar); border: 1px solid var(--border-subtle); border-radius: 9999px; font-size: 0.85rem; font-weight: 600; color: var(--text-accent); margin-bottom: 1rem; }
.hero-title { font-size: 3rem; font-weight: 900; letter-spacing: -0.02em; margin-bottom: 1rem; }
.hero-subtitle { font-size: 1.25rem; color: var(--text-muted); max-width: 750px; margin: 0 auto 2rem auto; }

.stats-grid { display: flex; justify-content: center; gap: 2rem; flex-wrap: wrap; margin-top: 2rem; }
.stat-card {
    background: var(--bg-card);
    border: 1px solid var(--border-card);
    border-radius: 12px;
    padding: 1.25rem 2rem;
    display: flex;
    flex-direction: column;
    align-items: center;
    box-shadow: var(--shadow-sm);
    min-width: 140px;
}
.stat-num { font-size: 2rem; font-weight: 800; color: var(--text-accent); }
.stat-label { font-size: 0.85rem; color: var(--text-muted); font-weight: 500; }

.modules-section { margin-top: 3rem; }
.section-header { margin-bottom: 2rem; }
.section-header h2 { font-size: 2rem; font-weight: 800; margin-bottom: 0.5rem; }
.section-header p { color: var(--text-muted); font-size: 1.05rem; }

.module-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(320px, 1fr)); gap: 1.5rem; }
.module-card {
    background: var(--bg-card);
    border: 1px solid var(--border-card);
    border-radius: 12px;
    padding: 1.5rem;
    display: flex;
    flex-direction: column;
    box-shadow: var(--shadow-sm);
    transition: transform 0.15s, box-shadow 0.15s, border-color 0.15s;
    color: inherit !important;
}
.module-card:hover {
    transform: translateY(-2px);
    box-shadow: var(--shadow-card);
    border-color: var(--text-accent);
    text-decoration: none;
}
.module-card-header { display: flex; align-items: center; gap: 1rem; margin-bottom: 1rem; }
.module-icon { font-size: 2rem; }
.module-name { font-size: 1.25rem; font-weight: 700; color: var(--text-main); }
.module-category { font-size: 0.8rem; color: var(--text-muted); }
.module-desc { font-size: 0.9rem; color: var(--text-muted); margin-bottom: 1.5rem; flex-grow: 1; }
.module-card-footer { display: flex; align-items: center; justify-content: space-between; border-top: 1px solid var(--border-subtle); padding-top: 0.75rem; }
.view-arrow { color: var(--text-accent); font-weight: bold; }

/* Site Footer */
.site-footer {
    border-top: 1px solid var(--border-subtle);
    background-color: var(--bg-surface);
    padding: 2.5rem 1.5rem;
    margin-top: 4rem;
    text-align: center;
}
.footer-inner p { font-size: 0.9rem; color: var(--text-muted); margin-bottom: 0.25rem; }
.footer-sub { font-size: 0.8rem; opacity: 0.8; }

/* Mobile Responsiveness */
@media (max-width: 900px) {
    .sidebar { display: none; position: fixed; z-index: 99; left: 0; width: 260px; box-shadow: var(--shadow-card); }
    .sidebar.open { display: block; }
    .mobile-toggle { display: block; }
    .content-main { padding: 1.5rem 1rem; }
    .hero-title { font-size: 2.25rem; }
    .hero-subtitle { font-size: 1.05rem; }
}
"#
}

pub fn get_search_js(search_index_json: &str) -> String {
    format!(
        r#"// Syntra Documentation Engine: Instant Fuzzy Search & Theme Toggle
const SEARCH_INDEX = {search_index_json};

document.addEventListener('DOMContentLoaded', () => {{
    initTheme();
    initSearch();
    initMobileNav();
}});

function initTheme() {{
    const savedTheme = localStorage.getItem('syntra-doc-theme') || 
        (window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light');
    setTheme(savedTheme);

    const toggle = document.getElementById('themeToggle');
    if (toggle) {{
        toggle.addEventListener('click', () => {{
            const current = document.documentElement.getAttribute('data-theme') || 'light';
            const next = current === 'dark' ? 'light' : 'dark';
            setTheme(next);
        }});
    }}
}}

function setTheme(theme) {{
    document.documentElement.setAttribute('data-theme', theme);
    localStorage.setItem('syntra-doc-theme', theme);
    const toggle = document.getElementById('themeToggle');
    if (toggle) {{
        toggle.textContent = theme === 'dark' ? '☀️' : '🌙';
    }}
}}

function initMobileNav() {{
    const toggle = document.getElementById('mobileToggle');
    const sidebar = document.getElementById('sidebar');
    if (toggle && sidebar) {{
        toggle.addEventListener('click', () => {{
            sidebar.classList.toggle('open');
        }});
    }}
}}

function initSearch() {{
    const input = document.getElementById('searchInput');
    const dropdown = document.getElementById('searchResults');
    if (!input || !dropdown) return;

    let selectedIndex = -1;
    let currentResults = [];

    // Global keyboard shortcut '/' to focus search
    window.addEventListener('keydown', (e) => {{
        if (e.key === '/' && document.activeElement !== input) {{
            e.preventDefault();
            input.focus();
            input.select();
        }}
    }});

    input.addEventListener('input', () => {{
        const query = input.value.trim().toLowerCase();
        if (query.length < 1) {{
            dropdown.classList.add('hidden');
            dropdown.innerHTML = '';
            selectedIndex = -1;
            return;
        }}

        currentResults = SEARCH_INDEX.filter(item => {{
            const nameMatch = item.name.toLowerCase().includes(query);
            const parentMatch = item.parent && item.parent.toLowerCase().includes(query);
            const modMatch = item.module.toLowerCase().includes(query);
            const descMatch = item.desc && item.desc.toLowerCase().includes(query);
            return nameMatch || parentMatch || modMatch || descMatch;
        }}).slice(0, 15);

        if (currentResults.length === 0) {{
            dropdown.innerHTML = '<div style="padding: 1rem; color: var(--text-muted); font-size: 0.85rem; text-align: center;">No matching symbols found.</div>';
            dropdown.classList.remove('hidden');
            selectedIndex = -1;
            return;
        }}

        selectedIndex = 0;
        renderSearchResults(dropdown, currentResults, selectedIndex);
        dropdown.classList.remove('hidden');
    }});

    input.addEventListener('keydown', (e) => {{
        if (dropdown.classList.contains('hidden') || currentResults.length === 0) return;

        if (e.key === 'ArrowDown') {{
            e.preventDefault();
            selectedIndex = (selectedIndex + 1) % currentResults.length;
            highlightSelected(dropdown, selectedIndex);
        }} else if (e.key === 'ArrowUp') {{
            e.preventDefault();
            selectedIndex = (selectedIndex - 1 + currentResults.length) % currentResults.length;
            highlightSelected(dropdown, selectedIndex);
        }} else if (e.key === 'Enter') {{
            e.preventDefault();
            if (selectedIndex >= 0 && selectedIndex < currentResults.length) {{
                window.location.href = currentResults[selectedIndex].url;
            }}
        }} else if (e.key === 'Escape') {{
            dropdown.classList.add('hidden');
            input.blur();
        }}
    }});

    // Click outside to dismiss
    document.addEventListener('click', (e) => {{
        if (!e.target.closest('.search-container')) {{
            dropdown.classList.add('hidden');
        }}
    }});
}}

function renderSearchResults(dropdown, results, selectedIdx) {{
    let html = '';
    results.forEach((item, idx) => {{
        const isSel = idx === selectedIdx ? ' selected' : '';
        const parentDisplay = item.parent ? `${{item.parent}}.` : '';
        html += `
        <a href="${{item.url}}" class="search-item${{isSel}}">
            <div class="search-item-header">
                <span class="search-item-title">${{parentDisplay}}${{item.name}}</span>
                <span class="search-item-meta"><span class="kind-tag ${{item.kind}}">${{item.kind}}</span> in ${{item.module}}</span>
            </div>
            ${{item.desc ? `<div class="search-item-desc">${{item.desc}}</div>` : ''}}
        </a>`;
    }});
    dropdown.innerHTML = html;
}}

function highlightSelected(dropdown, selectedIdx) {{
    const items = dropdown.querySelectorAll('.search-item');
    items.forEach((el, idx) => {{
        if (idx === selectedIdx) {{
            el.classList.add('selected');
            el.scrollIntoView({{ block: 'nearest' }});
        }} else {{
            el.classList.remove('selected');
        }}
    }});
}}

function copySnippet(btn) {{
    const pre = btn.closest('.code-card').querySelector('pre code');
    if (!pre) return;
    navigator.clipboard.writeText(pre.innerText).then(() => {{
        const orig = btn.textContent;
        btn.textContent = '✓ Copied';
        setTimeout(() => {{ btn.textContent = orig; }}, 1800);
    }}).catch(() => {{
        btn.textContent = 'Error';
    }});
}}
"#
    )
}
