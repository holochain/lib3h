extern crate proc_macro;

#[proc_macro_attribute]
pub fn spanned(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as MyMacroInput);

    /* ... */
}
