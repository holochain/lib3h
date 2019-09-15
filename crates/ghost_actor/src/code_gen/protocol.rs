use inflector::Inflector;
use proc_macro2::TokenStream;
use quote::ToTokens;

use std::collections::BTreeMap;

use crate::prelude::*;

#[derive(Debug)]
struct DefEvent {
    pub is_actor: bool,
    pub short_name: String,
    pub tall_name: String,
    pub evt_id: String,
    pub evt_name: String,
    pub evt_type: TokenStream,
}

#[derive(Debug)]
struct DefRequest {
    pub is_actor: bool,
    pub short_name: String,
    pub tall_name: String,
    pub req_id: String,
    pub req_name: String,
    pub req_type: TokenStream,
    pub res_id: String,
    pub res_name: String,
    pub res_type: TokenStream,
}

#[derive(Debug)]
enum DefType {
    Event(DefEvent),
    Request(DefRequest),
}

#[derive(Debug)]
struct DefProtocol {
    pub short_prefix: String,
    pub tall_prefix: String,
    pub scream_prefix: String,
    pub items: BTreeMap<String, DefType>,
}

fn tx_fields(x: &syn::Fields) -> Vec<TokenStream> {
    let mut out = Vec::new();
    if let syn::Fields::Unnamed(syn::FieldsUnnamed {
        paren_token: _,
        unnamed: fields,
    }) = x
    {
        for f in fields {
            let ty = &f.ty;
            out.push(quote!(#ty));
        }
    } else {
        panic!("ghost protocol unexpected {}", quote!(#x).to_string());
    }
    out
}

fn build_evt(definition: &mut DefProtocol, x: &syn::Fields, is_actor: bool) {
    let fields = tx_fields(x);
    assert_eq!(2, fields.len());
    let short_name = fields[0].to_string().to_snake_case();
    let tall_name = short_name.clone().to_pascal_case();
    let evt_type = fields[1].clone();
    let t = if is_actor { "actor" } else { "owner" };
    let tt = if is_actor { "Actor" } else { "Owner" };
    let evt_id = format!("event_to_{}_{}", t, short_name);
    let evt_name = format!("{}EventTo{}{}", definition.tall_prefix, tt, tall_name);
    let event = DefType::Event(DefEvent {
        is_actor,
        short_name,
        tall_name,
        evt_id: evt_id.clone(),
        evt_name,
        evt_type,
    });
    if definition.items.insert(evt_id.clone(), event).is_some() {
        panic!("{} already added", evt_id);
    }
}

fn build_req(definition: &mut DefProtocol, x: &syn::Fields, is_actor: bool) {
    let fields = tx_fields(x);
    assert_eq!(3, fields.len());
    let short_name = fields[0].to_string().to_snake_case();
    let tall_name = short_name.clone().to_pascal_case();
    let t = if is_actor { "actor" } else { "owner" };
    let tt = if is_actor { "Actor" } else { "Owner" };
    let req_type = fields[1].clone();
    let req_id = format!("request_to_{}_{}", t, short_name);
    let req_name = format!("{}RequestTo{}{}", definition.tall_prefix, tt, tall_name);
    let res_type = fields[2].clone();
    let res_id = format!("request_to_{}_{}_response", t, short_name);
    let res_name = format!(
        "{}RequestTo{}{}Response",
        definition.tall_prefix, tt, tall_name
    );
    let request = DefType::Request(DefRequest {
        is_actor,
        short_name,
        tall_name,
        req_id: req_id.clone(),
        req_name,
        req_type,
        res_id,
        res_name,
        res_type,
    });
    if definition.items.insert(req_id.clone(), request).is_some() {
        panic!("{} already added", req_id);
    }
}

impl DefProtocol {
    pub fn new(tokens: TokenStream) -> Self {
        let mut definition = Self {
            short_prefix: "".to_string(),
            tall_prefix: "".to_string(),
            scream_prefix: "".to_string(),
            items: BTreeMap::new(),
        };

        let parsed: syn::ItemEnum = syn::parse2(quote!(enum stub {#tokens})).unwrap();

        for v in &parsed.variants {
            match v.ident.to_string().as_str() {
                "prefix" => {
                    let fields = tx_fields(&v.fields);
                    assert_eq!(1, fields.len());
                    let short = fields[0].to_string().to_snake_case();
                    let tall = short.clone().to_pascal_case();
                    let scream = short.clone().to_screaming_snake_case();
                    definition.short_prefix = short;
                    definition.tall_prefix = tall;
                    definition.scream_prefix = scream;
                }
                "event_to_actor" => {
                    build_evt(&mut definition, &v.fields, true);
                }
                "event_to_owner" => {
                    build_evt(&mut definition, &v.fields, false);
                }
                "request_to_actor" => {
                    build_req(&mut definition, &v.fields, true);
                }
                "request_to_owner" => {
                    build_req(&mut definition, &v.fields, false);
                }
                _ => panic!("unknown ghost_protocol command: {}", v.ident),
            }
        }

        definition
    }
}

fn render_protocol(definition: &DefProtocol) -> TokenStream {
    let mut variants = Vec::new();

    for (_, item) in definition.items.iter() {
        match item {
            DefType::Event(evt) => {
                let name = format_ident!("{}", evt.evt_name);
                let evt_type = &evt.evt_type;
                variants.push(quote!(#name(#evt_type)));
            }
            DefType::Request(req) => {
                let req_name = format_ident!("{}", req.req_name);
                let req_type = &req.req_type;
                variants.push(quote!(#req_name(#req_type)));
                let res_name = format_ident!("{}", req.res_name);
                let res_type = &req.res_type;
                variants.push(quote!(#res_name(#res_type)));
            }
        }
    }

    let name = format_ident!("{}Protocol", definition.tall_prefix);
    quote! {
        ///main enum describing this protocol
        pub enum #name {
            #(#variants),*
        }
    }
}

#[derive(Debug)]
pub struct Protocol {
    definition: DefProtocol,
    rendered: TokenStream,
}

impl Protocol {
    pub fn new(tokens: TokenStream) -> Self {
        let definition = DefProtocol::new(tokens);

        let protocol = render_protocol(&definition);

        Self {
            definition,
            // stub
            rendered: quote! {
                #protocol
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
        let protocol = Protocol::new(quote! {
            prefix(test_proto),
            event_to_actor(print, String),
            request_to_actor(add_1, i32, Result<i32, ()>),
            event_to_owner(print, String),
            request_to_owner(add_1, i32, Result<i32, ()>),
        });

        //println!("{:#?}", protocol);
        println!(
            "{}",
            code_gen::try_fmt(protocol.to_token_stream().to_string())
        );
        let rendered = protocol.into_token_stream();

        assert_eq!(
            rendered.to_string(),
            quote! {
                #[doc = r"main enum describing this protocol"]
                pub enum TestProtoProtocol {
                    TestProtoEventToActorPrint(String),
                    TestProtoEventToOwnerPrint(String),
                    TestProtoRequestToActorAdd1(i32),
                    TestProtoRequestToActorAdd1Response(Result<i32, ()>),
                    TestProtoRequestToOwnerAdd1(i32),
                    TestProtoRequestToOwnerAdd1Response(Result<i32, ()>)
                }
            }
            .to_string(),
        );
    }
}
