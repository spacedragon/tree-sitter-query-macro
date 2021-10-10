use tree_sitter::{Node, Parser};
use tree_sitter_query_macro::make_query;
fn main() {
    
    let code = include_str!("../Test.java");
    dbg!(code);
    let mut parser = Parser::new();
        parser
            .set_language(tree_sitter_java::language())
            .expect("Error loading Java grammar");
    let tree = parser.parse(code, None).unwrap();
    let node = tree.root_node();

    let capture_class = |node: &Node| -> bool {
        dbg!(node);
        true
    };

    let mut query = make_query!{
        (program
            (class_declaration) @capture_class
        )
    };

    query(&node);
}
