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
    pub evt_d_list_idx: usize,
}

#[derive(Debug)]
struct DefRequest {
    pub is_actor: bool,
    pub short_name: String,
    pub tall_name: String,
    pub req_id: String,
    pub req_name: String,
    pub req_type: TokenStream,
    pub req_d_list_idx: usize,
    pub res_id: String,
    pub res_name: String,
    pub res_type: TokenStream,
    pub res_d_list_idx: usize,
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
    pub protocol_name: String,
    pub d_list_name: String,
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
        evt_d_list_idx: 0,
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
        req_d_list_idx: 0,
        res_id,
        res_name,
        res_type,
        res_d_list_idx: 0,
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
            protocol_name: "".to_string(),
            d_list_name: "".to_string(),
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

        definition.protocol_name = format!("{}Protocol", &definition.tall_prefix);
        definition.d_list_name = format!("{}_D_LIST", &definition.scream_prefix);

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

    let name = format_ident!("{}", definition.protocol_name);
    quote! {
        ///main enum describing this protocol
        #[derive(Debug, Clone)]
        pub enum #name {
            #(#variants),*
        }
    }
}

fn proto_dest(is_actor: bool) -> TokenStream {
    let dest = if is_actor {
        format_ident!("Actor")
    } else {
        format_ident!("Owner")
    };
    quote!(::ghost_actor::GhostProtocolDestination::#dest)
}

fn proto_v_type(v_type: &str) -> TokenStream {
    let v_type = format_ident!("{}", v_type);
    quote!(::ghost_actor::GhostProtocolVariantType::#v_type)
}

fn render_d_list(definition: &mut DefProtocol) -> TokenStream {
    let mut d_list = Vec::new();

    let mut next_index = 0_usize;

    for (_, item) in definition.items.iter_mut() {
        match item {
            DefType::Event(evt) => {
                let id = &evt.evt_id;
                let dest = proto_dest(evt.is_actor);
                let v_type = proto_v_type("Event");
                evt.evt_d_list_idx = next_index;
                next_index += 1;
                d_list.push(quote! {
                    ::ghost_actor::GhostProtocolDiscriminant {
                        id: #id,
                        destination: #dest,
                        variant_type: #v_type,
                    }
                });
            }
            DefType::Request(req) => {
                let id = &req.req_id;
                let dest = proto_dest(req.is_actor);
                let v_type = proto_v_type("Request");
                req.req_d_list_idx = next_index;
                next_index += 1;
                d_list.push(quote! {
                    ::ghost_actor::GhostProtocolDiscriminant {
                        id: #id,
                        destination: #dest,
                        variant_type: #v_type,
                    }
                });
                let id = &req.res_id;
                let dest = proto_dest(!req.is_actor);
                let v_type = proto_v_type("Response");
                req.res_d_list_idx = next_index;
                next_index += 1;
                d_list.push(quote! {
                    ::ghost_actor::GhostProtocolDiscriminant {
                        id: #id,
                        destination: #dest,
                        variant_type: #v_type,
                    }
                });
            }
        }
    }

    let name = format_ident!("{}", definition.d_list_name);
    quote! {
        ///discriminant list meta data about this protocol
        static #name: &'static [::ghost_actor::GhostProtocolDiscriminant] = &[
            #(#d_list),*
        ];
    }
}

fn render_ghost_protocol(definition: &DefProtocol) -> TokenStream {
    let protocol_name = format_ident!("{}", definition.protocol_name);
    let d_list_name = format_ident!("{}", definition.d_list_name);

    let mut arms = Vec::new();

    for (_, item) in definition.items.iter() {
        match item {
            DefType::Event(evt) => {
                let name = format_ident!("{}", evt.evt_name);
                let index = evt.evt_d_list_idx;
                arms.push(quote!(#protocol_name::#name(_) => &#d_list_name[#index]));
            }
            DefType::Request(req) => {
                let name = format_ident!("{}", req.req_name);
                let index = req.req_d_list_idx;
                arms.push(quote!(#protocol_name::#name(_) => &#d_list_name[#index]));
                let name = format_ident!("{}", req.res_name);
                let index = req.res_d_list_idx;
                arms.push(quote!(#protocol_name::#name(_) => &#d_list_name[#index]));
            }
        }
    }

    quote! {
        impl ::ghost_actor::GhostProtocol for #protocol_name {
            fn discriminant_list() -> &'static [::ghost_actor::GhostProtocolDiscriminant] {
                #d_list_name
            }

            fn discriminant(&self) -> &::ghost_actor::GhostProtocolDiscriminant {
                match self {
                    #(#arms),*
                }
            }
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
        let mut definition = DefProtocol::new(tokens);

        // do this first so the d_list indexes get updated
        let d_list = render_d_list(&mut definition);

        let protocol = render_protocol(&definition);
        let ghost_protocol = render_ghost_protocol(&definition);

        Self {
            definition,
            // stub
            rendered: quote! {
                #protocol
                #d_list
                #ghost_protocol
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

        /*
        assert_eq!(
            protocol.into_token_stream().to_string(),
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
        */
    }
}
