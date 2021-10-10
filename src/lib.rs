use proc_macro2::TokenStream;
use quote::quote;
use std::vec;
use syn::{
    ext::IdentExt,
    parenthesized,
    parse::{Parse, ParseBuffer, ParseStream},
    token, Error, Ident, Token,
};
 
#[derive(Debug)]
enum Field {
    Named(Ident),
    Index(usize),
}

#[derive(Debug)]
enum Logical {
    Alternative,
    Sequence,
    Single,
}

#[derive(Debug)]
struct SubQuery {
    field: Option<Field>,
    logic: Logical,
    queries: Vec<Box<Query>>,
}

#[derive(Default, Debug)]
struct Query {
    node: Option<Ident>,
    children: Vec<SubQuery>,
    capture_name: Option<Ident>,
    is_optional: bool,
    is_star: bool,
    is_plus: bool,
}

impl Parse for Query {
    fn parse(stream: ParseStream) -> syn::Result<Self> {
        let result = parse_query(&stream)?;
        dbg!(&result);
        Ok(result)
    }
}

fn parse_field(stream: &ParseBuffer) -> syn::Result<Option<Field>> {
    if stream.peek(Ident::peek_any) {
        let field = stream.call(Ident::parse_any)?;
        let _: Token!(:) = stream.parse()?;
        Ok(Some(Field::Named(field)))
    } else if stream.peek(syn::LitInt) {
        let idx: syn::LitInt = stream.parse()?;
        let idx: usize = idx.base10_parse()?;
        let _: Token!(:) = stream.parse()?;
        Ok(Some(Field::Index(idx)))
    } else {
        Ok(None)
    }
}

fn parse_queries(stream: &ParseBuffer) -> Vec<Box<Query>> {
    let mut queries = vec![];
    while let Ok(query) = parse_query(stream) {
        queries.push(Box::new(query));
    }
    queries
}

fn parse_child(stream: &ParseBuffer, query: &mut Query) -> syn::Result<()> {
    let field = parse_field(stream)?;

    if stream.peek(token::Bracket) {
        let child_content;
        let _ = syn::bracketed!(child_content in stream);
        let logic = Logical::Alternative;
        let queries = parse_queries(&child_content);
        query.children.push(SubQuery {
            field,
            logic,
            queries,
        })
    } else if stream.peek(token::Paren) {
        let child_content;
        let _ = parenthesized!(child_content in stream);

        if child_content.peek(token::Paren) {
            let logic = Logical::Sequence;
            let queries = parse_queries(&child_content);
            query.children.push(SubQuery {
                field,
                logic,
                queries,
            })
        } else {
            let mut sub_query = Query::default();
            parse_query_content(child_content, &mut sub_query)?;
            parse_query_tail(stream, &mut sub_query)?;
            query.children.push(SubQuery {
                field,
                logic: Logical::Single,
                queries: vec![Box::new(sub_query)],
            })
        }
    } else {
        return Err(Error::new(stream.span(), "unexpect token"));
    }
    Ok(())
}

fn parse_query(stream: &ParseBuffer) -> syn::Result<Query> {
    if stream.is_empty() {
        return Err(Error::new(stream.span(), "nothing"));
    }

    let mut result = Query::default();
    let content;
    let _ = parenthesized!(content in stream);

    parse_query_content(content, &mut result)?;
    parse_query_tail(stream, &mut result)?;
    Ok(result)
}

fn parse_query_content(mut content: ParseBuffer, query: &mut Query) -> Result<(), Error> {
    
    if content.peek(Token!(_)) {
        let _: Token!(_) = content.parse()?;
        query.node = None;
    } else {
        query.node = content.parse()?;
    }

    while parse_child(&mut content, query).is_ok() {
        // children
    }
    Ok(())
}

fn parse_query_tail(stream: &ParseBuffer, query: &mut Query) -> Result<(), Error> {
    if stream.peek(Token!(+)) {
        let _: Token!(+) = stream.parse()?;
        query.is_plus = true;
    }
    if stream.peek(Token!(*)) {
        let _: Token!(*) = stream.parse()?;
        query.is_star = true;
    }
    if stream.peek(Token!(?)) {
        let _: Token!(?) = stream.parse()?;
        query.is_optional = true;
    }
    Ok(if stream.peek(Token!(@)) {
        let _: Token!(@) = stream.parse()?;
        let cap: Ident = stream.parse()?;
        query.capture_name = Some(cap);
    })
}

#[proc_macro]
pub fn make_query(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let query = syn::parse_macro_input!(input as Query);
    build_query(&query).into() 
}

fn build_query(query: &Query) -> TokenStream {    
    let mut parts = vec![];

    if let Some(kind) = &query.node {
        parts.push(quote!{ node.kind() == stringify!( #kind ) })
    } else {
        parts.push(quote!{ true })
    }

    let mut decl_subs = quote! {};
    if !query.children.is_empty() {
        let test_children = build_children(&query.children);
        decl_subs = quote! { let mut test_children = #test_children; };
        parts.push(quote! {test_children(node)});
    }

    if let Some(capture) = &query.capture_name {
        parts.push(quote! {
            #capture(node)
        });
    };

    let q = quote! {
        |node: &tree_sitter::Node| -> bool {    
             #decl_subs;
             #( #parts )&&*
        }
    };
    q
}

fn build_children(children: &Vec<SubQuery>) -> TokenStream {
    let mut subs= vec![];
    let mut tests= vec![];
    for (idx, sub) in children.iter().enumerate() {
        let sub_name = quote::format_ident!("child_query_{}", idx);
        let child_query = build_child_query(&sub);
        let sub = if let Some(field) = &sub.field {
            match field {
                Field::Named(name) => {
                    quote! {
                        let mut #sub_name = |node: &tree_sitter::Node| -> bool {
                            let mut child_query = #child_query;
                            node.child_by_field_name(stringify!(#name))
                                .map(|n| child_query(&n))
                                .unwrap_or(false)
                        };
                    }
                }
                Field::Index(idx) => {
                    quote! {
                        let mut #sub_name = |node: &tree_sitter::Node| -> bool {
                            let mut child_query = #child_query;
                            node.named_child(#idx)
                                .map(|n| child_query(&n))
                                .unwrap_or(false)
                        };
                    }
                }
            } 
        } else {
            quote! {
                let mut #sub_name = |node: &tree_sitter::Node| -> bool {
                    let mut child_query = #child_query;
                    (0..node.named_child_count())
                    .map(|i|node.named_child(i).unwrap())
                    .any(|n| child_query(&n))
                };
            }
        };
        subs.push(sub);
        tests.push(quote! {
            #sub_name(node)
        })
    }
    quote! {
        |node: &tree_sitter::Node | -> bool {
            #( #subs )*;
            #( #tests )&&*
        }
    }
}

fn build_child_query(subquery: &SubQuery) -> TokenStream {
    match subquery.logic {
        Logical::Alternative => {
            let mut queries = vec![];
            let mut testers = vec![];
            for (idx, q) in subquery.queries.iter().enumerate() {
                let c = build_query(q.as_ref());
                let fn_name = quote::format_ident!("alt_child_query_{}", idx);
                let query = quote! {
                    let #fn_name = #c;
                };
                queries.push(query);
                let tester = quote! {
                    #fn_name(child)
                };
                
                testers.push(tester);
            }
            quote! {
                |child: &tree_sitter::Node| -> bool {
                    #( #queries )*
                    #( #testers )||*
                        
                }
            }
        }
        Logical::Sequence => {
            let mut queries = vec![];
            let mut testers = vec![];
            let len = subquery.queries.len();
            for (idx, q) in subquery.queries.iter().enumerate() {
                let c = build_query(q.as_ref());
                let fn_name = quote::format_ident!("seq_child_query_{}", idx);
                let query = quote! {
                    let #fn_name = #c;
                };
                queries.push(query);
                let tester = quote! {
                    #fn_name(&node.named_child(i + #idx))
                };
                testers.push(tester);
            }
            quote! {
                |child: &tree_sitter::Node| -> bool {
                    #( #queries )*
                    for i in (0 .. (node.named_child_count()-#len)) {
                        if #( #testers )&&* {
                            return true;
                        }
                    }
                    return false; 
                }
            }
        },
        Logical::Single => {
            if let Some(q) = subquery.queries.first() {
                let c = build_query(q.as_ref());
                quote! {
                    |child: &tree_sitter::Node| -> bool {
                            let mut query = #c;
                            query(child)
                    }
                }
            } else {
                quote! { true }
            }
            
        },
    }
    
}