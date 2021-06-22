extern crate proc_macro;
use {arcana_timespan::TimeSpan, proc_macro::TokenStream};

#[proc_macro]
pub fn timespan(item: TokenStream) -> TokenStream {
    match item.to_string().parse::<TimeSpan>() {
        Ok(span) => format!("arcana::TimeSpan::from_micros({})", span.as_micros()),
        Err(err) => format!("compile_error!(\"{}\")", err),
    }
    .parse()
    .unwrap()
}
