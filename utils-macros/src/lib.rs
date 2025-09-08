//! Very very bad code (and very unoptimized) for what is very not necessary :)
//! (i am not familiar with proc macros)

use std::iter::zip;

use darling::{FromDeriveInput, FromField};
use proc_macro::TokenStream;
use quote::quote;
use syn::parse_quote;
use convert_case::{Casing, Case};

fn ident_to_pascal(ident: &syn::Ident) -> syn::Ident {
    let s = ident.to_string().to_case(Case::Pascal);
    syn::Ident::new(&s, ident.span())
}

#[derive(darling::FromDeriveInput)]
#[darling(default, attributes(try_clone))]
struct Opts {
    error_enum_name: Option<syn::Ident>,
    error_type: Option<syn::Type>,
    derive_thiserror: bool,
}

impl Default for Opts {
    fn default() -> Self {
        Self {
            error_enum_name: None,
            error_type: None,
            derive_thiserror: true
        }
    }
}

#[derive(darling::FromField)]
#[darling(default, attributes(try_clone))]
struct FieldsOpts {
    use_clone: bool,
    use_try_clone: bool,
    error_type: Option<syn::Type>,
    clone_with: Option<syn::Expr>,
}

impl Default for FieldsOpts {
    fn default() -> Self {
        Self {
            use_clone: false,
            use_try_clone: false,
            clone_with: None,
            error_type: None,
        }
    }
}

#[proc_macro_derive(TryClone, attributes(try_clone))]
pub fn derive_try_clone(item: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(item as syn::DeriveInput);
    let opts = Opts::from_derive_input(&input).expect("Wrong options");

    let ident = &input.ident;
    let vis = &input.vis;
    let generics = &input.generics;

    let try_clone_enum_name_ = opts.error_enum_name
        .unwrap_or_else(|| quote::format_ident!("{ident}TryCloneError"));
    let mut provides_error_type = opts.error_type.is_some();
    let mut error_type = opts.error_type
        .unwrap_or_else(|| parse_quote!{ #try_clone_enum_name_ });
    let try_clone = quote!{ ::utils::try_clone::TryClone };

    match &input.data {
        syn::Data::Struct(syn::DataStruct { fields, .. }) => {
            let mut error_fields = quote!{ };
            let mut implementation = quote!{ };

            let mut try_clone_generics = generics.clone();
            let where_clause = try_clone_generics.make_where_clause();

            let enum_derives = if opts.derive_thiserror {
                quote! {
                    #[derive(Debug, ::thiserror::Error)]
                    #[automatically_derived]
                }
            }
            else { quote!{ } };
            let field_attrs = if opts.derive_thiserror {
                quote!{ #[error(transparent)] }
            } else { quote!{ } };

            let fields_opts = fields.iter()
                .map(|field| FieldsOpts::from_field(field).expect("Wrong options"))
                .collect::<Vec<_>>();
            
            let inverted_use_clone = fields_opts.iter()
                .any(|opt| opt.use_try_clone);
            let does_field_use_clone = |opt: &FieldsOpts| {
                let does_field_use_try_clone = (!inverted_use_clone || opt.use_try_clone) && !opt.use_clone;
                !does_field_use_try_clone
            };
            let try_clone_count = fields_opts.iter()
                .filter(|opt| !does_field_use_clone(opt)).count();

            // Fakes provided type when there is only a single try clone type
            if try_clone_count == 1 {
                let (field, field_opts) = zip(fields, &fields_opts)
                    .find(|(_, opts)| !does_field_use_clone(opts)).expect("There is one");
                let ty = &field.ty;

                provides_error_type = true;
                error_type = field_opts.error_type.clone()
                    .unwrap_or_else(|| parse_quote!{ <#ty as #try_clone>::Error });
            }
            if try_clone_count == 0 {
                provides_error_type = true;
                error_type = parse_quote! { ! };
            }

            for (field, field_opts) in zip(fields, fields_opts) {
                let ident = field.ident.as_ref().unwrap();
                let pascal_ident = ident_to_pascal(ident);
                let ty = &field.ty;

                let use_clone = does_field_use_clone(&field_opts);

                let field_has_error_type = field_opts.error_type.is_some();
                let clone_with = field_opts.clone_with.unwrap_or_else(|| parse_quote!{ #try_clone::try_clone });
                let field_error_type = field_opts.error_type.unwrap_or_else(|| parse_quote!{ <#ty as #try_clone>::Error });

                if use_clone {
                    implementation.extend(quote!{
                        #ident: <#ty as Clone>::clone(&self.#ident),
                    });
                }
                else if provides_error_type {
                    implementation.extend(quote!{
                        #ident: (#clone_with)(&self.#ident)
                            .map_err(|err| <#error_type as From<#field_error_type>>::from(err))?,
                    });
                }
                else {
                    implementation.extend(quote!{
                        #ident: (#clone_with)(&self.#ident)
                            .map_err(#error_type::#pascal_ident)?,
                    });
                }
                if !use_clone {
                    error_fields.extend(quote!{
                        #field_attrs
                        #pascal_ident(#field_error_type),
                    });
                }
                if !field_has_error_type {
                    where_clause
                        .predicates
                        .push(parse_quote!(#ty: #try_clone));
                }
                if opts.derive_thiserror {
                    where_clause.predicates.push(parse_quote!(#field_error_type: ::std::fmt::Debug));
                }
            }

            let (impl_generics, ty_generics, where_clause) = try_clone_generics.split_for_impl();

            let enum_define = if !provides_error_type {quote!{
                #[automatically_derived]
                #enum_derives
                #vis enum #try_clone_enum_name_ #try_clone_generics #where_clause {
                    #error_fields
                }

                #[automatically_derived]
                impl #impl_generics From<!> for #try_clone_enum_name_ #ty_generics #where_clause {
                    fn from(_: !) -> Self { unreachable!() }
                }
            }} else {
                quote!{}
            };

            let error_type_define = if !provides_error_type {
                quote! { #try_clone_enum_name_ #ty_generics }
            } else {
                quote! { #error_type }
            };

            quote! {
                #enum_define

                #[automatically_derived]
                impl #impl_generics #try_clone for #ident #ty_generics #where_clause {
                    type Error = #error_type_define;

                    fn try_clone(&self) -> Result<Self, Self::Error> {
                        Ok(Self {
                            #implementation
                        })
                    }
                }
            }
        },
        syn::Data::Enum(_data_enum) => todo!(),
        syn::Data::Union(_) => unimplemented!("Cannot clone a union"),
    }.into()
}
