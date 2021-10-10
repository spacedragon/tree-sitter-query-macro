##Do tree-sitter query directly by rust macro (WIP)

##Example
```rust
#[derive(Debug)]
struct Cons {
    modifiers: String,
    name: String,
    parameters: String,
    body: String
}

fn tree_sitter_macro(source: &str,node: Node) -> Cons {
    let mut result = Cons {
        modifiers: String::new(),
        name: String::new(),
        parameters: String::new(),
        body: String::new(),
    };
    // capture with rust closure
    let mut modifiers = |node: Node| -> bool {
        result.modifiers = node.utf8_text(source.as_bytes()).unwrap().to_string();
        true
    };

    let mut name = |node: Node| -> bool {
        result.name = node.utf8_text(source.as_bytes()).unwrap().to_string();
        true
    };
    let mut parameters = |node: Node| -> bool {
        result.parameters = node.utf8_text(source.as_bytes()).unwrap().to_string();
        true
    };
    let mut body = |node: Node| -> bool {
        result.body = node.utf8_text(source.as_bytes()).unwrap().to_string();
        true
    };

    let mut query = make_query!{
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
        )
    };
    query(node);
    result
}

```