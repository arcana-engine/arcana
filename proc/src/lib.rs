use proc_macro::TokenStream;

extern crate proc_macro;

mod time;
mod unfold;

#[proc_macro]
pub fn timespan(item: TokenStream) -> TokenStream {
    time::timespan(item)
}

#[proc_macro_derive(Unfold, attributes(unfold))]
pub fn unfold(item: TokenStream) -> TokenStream {
    match unfold::derive_unfold(item) {
        Ok(tokens) => tokens,
        Err(err) => err.into_compile_error().into(),
    }
}
