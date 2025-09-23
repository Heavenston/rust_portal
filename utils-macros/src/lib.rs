mod derive_try_clone;

use proc_macro::TokenStream;

#[proc_macro_derive(TryClone, attributes(try_clone))]
pub fn derive_try_clone(item: TokenStream) -> TokenStream {
    derive_try_clone::derive_try_clone(item)
}

