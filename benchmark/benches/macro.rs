use criterion::{black_box, criterion_group, criterion_main, Criterion};
#[macro_use]
extern crate lazy_static;
use tree_sitter::{Parser, Query, Node, QueryCursor};
use tree_sitter_query_macro::make_query;

lazy_static! {
    static ref CON_QUERY: Query = {
        Query::new(
            tree_sitter_java::language(),
            "
        (program
            (class_declaration
                body: (class_body    
                    (constructor_declaration 
                        (modifiers) @modifiers
                        name: (identifier) @name
                        parameters: (formal_parameters) @parameters
                        body: (constructor_body) @body
                    )
                )
            )
        )",
        ).unwrap()
    }; 
}

#[derive(Debug)]
struct Cons {
    modifiers: String,
    name: String,
    parameters: String,
    body: String
}

fn tree_sitter_macro(source: &str,node: Node) -> Cons {
    let mut modifiers= String::new();
    let mut name= String::new();
    let mut parameters= String::new();
    let mut body= String::new();

    let mut modifiers_fn = |node: &Node| -> bool {
        modifiers = node.utf8_text(source.as_bytes()).unwrap().to_string();
        true
    };

    let mut name_fn = |node: &Node| -> bool {
        name = node.utf8_text(source.as_bytes()).unwrap().to_string();
        true
    };
    let mut parameters_fn = |node: &Node| -> bool {
        parameters = node.utf8_text(source.as_bytes()).unwrap().to_string();
        true
    };
    let mut body_fn = |node: &Node| -> bool {
        body = node.utf8_text(source.as_bytes()).unwrap().to_string();
        true
    };

    let mut query = make_query!{
        (program
            (class_declaration
                body: (class_body    
                    (constructor_declaration 
                        (modifiers) @modifiers_fn
                        name: (identifier) @name_fn
                        parameters: (formal_parameters) @parameters_fn
                        body: (constructor_body) @body_fn
                    )
                )
            )
        )
    };
    query(&node);
    Cons {
        modifiers,
        name,
        parameters,
        body
    }
}

fn tree_sitter_query(source: &str,node: Node) -> Cons {
    let mut result = Cons {
        modifiers: String::new(),
        name: String::new(),
        parameters: String::new(),
        body: String::new(),
    };
    let mut query_cursor = QueryCursor::new();

    let mut matches = query_cursor.matches(&CON_QUERY, node, |_| source.as_bytes());
    if let Some(m) = matches.next() {
        for c in m.captures.iter() {
            let capture_name = &CON_QUERY.capture_names()[c.index as usize];
            let text = c.node.utf8_text(source.as_bytes()).unwrap();
            match capture_name.as_str() {
                "modifiers" => result.modifiers = text.to_string(),
                "name" => result.name = text.to_string(),
                "parameters" => result.parameters = text.to_string(),
                "body" => result.body = text.to_string(),
                _ => {}
            }
        }
    }
    return result;
}

fn fibonacci(n: u64) -> u64 {
    match n {
        0 => 1,
        1 => 1,
        n => fibonacci(n-1) + fibonacci(n-2),
    }
}

fn criterion_benchmark(c: &mut Criterion) {
    let code = include_str!("../Test.java");
    let mut parser = Parser::new();
        parser
            .set_language(tree_sitter_java::language())
            .expect("Error loading Java grammar");
    let tree = parser.parse(code, None).unwrap();
    let node = tree.root_node();
    c.bench_function("tree-sitter-query", |b| b.iter(|| tree_sitter_query(black_box(code), black_box(node))));
    c.bench_function("tree-sitter-macro", |b| b.iter(|| tree_sitter_macro(black_box(code), black_box(node))));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);