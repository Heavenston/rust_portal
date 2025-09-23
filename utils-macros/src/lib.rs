mod derive_try_clone;
mod optional_enum_fields;

use proc_macro::TokenStream;

#[proc_macro_derive(TryClone, attributes(try_clone))]
pub fn derive_try_clone(item: TokenStream) -> TokenStream {
    derive_try_clone::derive_try_clone(item)
}

#[proc_macro]
pub fn optional_enum_fields(item: TokenStream) -> TokenStream {
    optional_enum_fields::optional_enum_fields(item)
}
