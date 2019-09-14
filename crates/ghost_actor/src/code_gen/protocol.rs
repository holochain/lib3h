use proc_macro2::TokenStream;
use quote::ToTokens;
use inflector::Inflector;

use crate::prelude::*;

#[derive(Debug)]
struct DefEvent {
    pub short_name: TokenStream,
    pub tall_name: TokenStream,
    pub evt_type: TokenStream,
}

#[derive(Debug)]
struct DefRequest {
    pub short_name: TokenStream,
    pub tall_name: TokenStream,
    pub req_type: TokenStream,
    pub res_type: TokenStream,
}

#[derive(Debug)]
struct DefProtocol {
    pub short_prefix: TokenStream,
    pub tall_prefix: TokenStream,
    pub events_to_actor: Vec<DefEvent>,
    pub requests_to_actor: Vec<DefRequest>,
    pub events_to_owner: Vec<DefEvent>,
    pub requests_to_owner: Vec<DefRequest>,
}

fn tx_fields(x: &syn::Fields) -> Vec<TokenStream> {
    let mut out = Vec::new();
    if let syn::Fields::Unnamed(syn::FieldsUnnamed { paren_token: _, unnamed: fields }) = x {
        for f in fields {
            let ty = &f.ty;
            out.push(quote!(#ty));
        }
    } else {
        panic!("ghost protocol unexpected {}", quote!(#x).to_string());
    }
    out
}

fn build_evt(x: &syn::Fields) -> DefEvent {
    let fields = tx_fields(x);
    assert_eq!(2, fields.len());
    let short_name = fields[0].to_string().to_snake_case();
    let tall_name = short_name.clone().to_pascal_case();
    let evt_type = fields[1].clone();
    DefEvent {
        short_name: quote!(#short_name),
        tall_name: quote!(#tall_name),
        evt_type,
    }
}

fn build_req(x: &syn::Fields) -> DefRequest {
    let fields = tx_fields(x);
    assert_eq!(3, fields.len());
    let short_name = fields[0].to_string().to_snake_case();
    let tall_name = short_name.clone().to_pascal_case();
    let req_type = fields[1].clone();
    let res_type = fields[2].clone();
    DefRequest {
        short_name: quote!(#short_name),
        tall_name: quote!(#tall_name),
        req_type,
        res_type,
    }
}

impl DefProtocol {
    pub fn new(tokens: TokenStream) -> Self {
        let mut definition = Self {
            short_prefix: quote!(),
            tall_prefix: quote!(),
            events_to_actor: Vec::new(),
            requests_to_actor: Vec::new(),
            events_to_owner: Vec::new(),
            requests_to_owner: Vec::new(),
        };

        let parsed: syn::ItemEnum = syn::parse2(quote!(enum stub {#tokens})).unwrap();

        for v in &parsed.variants {
            match v.ident.to_string().as_str() {
                "prefix" => {
                    let fields = tx_fields(&v.fields);
                    assert_eq!(1, fields.len());
                    let short = fields[0].to_string().to_snake_case();
                    let tall = short.clone().to_pascal_case();
                    definition.short_prefix = quote!(#short);
                    definition.tall_prefix = quote!(#tall);
                }
                "event_to_actor" => {
                    definition.events_to_actor.push(build_evt(&v.fields));
                }
                "event_to_owner" => {
                    definition.events_to_owner.push(build_evt(&v.fields));
                }
                "request_to_actor" => {
                    definition.requests_to_actor.push(build_req(&v.fields));
                }
                "request_to_owner" => {
                    definition.requests_to_owner.push(build_req(&v.fields));
                }
                _ => panic!("unknown ghost_protocol command: {}", v.ident),
            }
        }

        definition
    }
}

pub struct Protocol {
    rendered: TokenStream,
}

impl Protocol {
    pub fn new(tokens: TokenStream) -> Self {
        let definition = DefProtocol::new(tokens);
        println!("{:#?}", definition);

        Self {
            // stub
            rendered: quote! {
                #[derive(Debug)]
                pub enum TestEnum {
                    TestVariant,
                }
            },
        }
    }
}

impl ToTokens for Protocol {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(self.rendered.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_renders_protocol() {
        let rendered = Protocol::new(quote! {
            prefix(test_proto),
            event_to_actor(print, String),
            request_to_actor(add_1, i32, Result<i32, ()>),
            event_to_owner(print, String),
            request_to_owner(add_1, i32, Result<i32, ()>),
        })
        .into_token_stream();

        assert_eq!(
            rendered.to_string(),
            quote! {
                #[derive(Debug)]
                pub enum TestEnum {
                    TestVariant,
                }
            }
            .to_string(),
        );
    }
}
