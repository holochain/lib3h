use proc_macro2::TokenStream;
use quote::ToTokens;

pub struct Protocol {
    rendered: TokenStream,
}

impl Protocol {
    pub fn new(tokens: TokenStream) -> Self {
        Self {
            // stub, just passing through tokens for now
            rendered: tokens,
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
            pub enum TestProtocol {}
        })
        .into_token_stream();

        assert_eq!(
            rendered.to_string(),
            quote! {
                pub enum TestProtocol {}
            }
            .to_string(),
        );
    }
}
