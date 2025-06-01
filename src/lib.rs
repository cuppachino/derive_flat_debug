use proc_macro::TokenStream;
use quote::{ quote };
use syn::{ parse_macro_input, Attribute, Data, DeriveInput, Fields, Meta, Type };

#[proc_macro_derive(DebugFlat, attributes(debug))]
pub fn derive_debug_flat(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let debug_impl = match &input.data {
        Data::Enum(data_enum) => {
            let variants = data_enum.variants.iter().map(|variant| {
                let variant_name = &variant.ident;
                let variant_str = variant_name.to_string();
                let skip_simplify = has_debug_skip(&variant.attrs);
                let has_debug_flatten = has_debug_flatten(&variant.attrs);

                if skip_simplify && has_debug_flatten {
                    panic!("Cannot use both `#[debug(skip)]` and `#[debug(flatten)]` on the same variant: {}", variant_name);
                }

                match &variant.fields {
                    // Unit variant
                    Fields::Unit => {
                        quote! {
                            #name::#variant_name => f.write_str(#variant_str),
                        }
                    }
                    // Single field variant
                    Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                        let field = &fields.unnamed[0];

                        if has_debug_skip(&field.attrs) {
                            quote! {
                                #name::#variant_name(_) => f.write_str(#variant_str),
                            }
                        } else {
                            // Check if the field type name matches the variant name
                            let should_omit_variant =
                                !skip_simplify &&
                                ((if let Type::Path(type_path) = &field.ty {
                                    if let Some(last_segment) = type_path.path.segments.last() {
                                        last_segment.ident.to_string() == variant_str
                                    } else {
                                        false
                                    }
                                } else {
                                    false
                                }) || has_debug_flatten);

                            if should_omit_variant {
                                let type_prefix = if let Type::Path(type_path) = &field.ty {
                                    type_path.path.segments
                                        .last()
                                        .map(|s| s.ident.to_string())
                                        .unwrap_or_default()
                                } else {
                                    String::new()
                                };

                                quote! {
                                    #name::#variant_name(value) => {
                                        let s = if f.alternate() {
                                            format!("{:#?}", value)
                                        } else {
                                            format!("{:?}", value)
                                        };
                                        if let Some(stripped) = s.strip_prefix(&#type_prefix) {
                                            // write the name of the variant instead at the start
                                            f.write_str(#variant_str)?;
                                            f.write_str(stripped)
                                        } else {
                                            std::fmt::Debug::fmt(value, f)
                                        }
                                        
                                    },
                                }
                            } else {
                                quote! {
                                    #name::#variant_name(value) => {
                                        let mut dbg = f.debug_tuple(#variant_str);
                                        dbg.field(value);
                                        dbg.finish()
                                    },
                                }
                            }
                        }
                    }
                    // Multiple fields or named fields - always show variant name
                    Fields::Unnamed(fields) => {
                        let (field_patterns, field_debugs) = fields.unnamed
                            .iter()
                            .enumerate()
                            .map(|(i, field)| {
                                let pattern = syn::Ident::new(
                                    &format!("field_{}", i),
                                    proc_macro2::Span::call_site()
                                );

                                let quoted = if has_debug_skip(&field.attrs) {
                                    quote! { dbg.field(&_); }
                                } else {
                                    quote! { dbg.field(&#pattern); }
                                };

                                (pattern, quoted)
                            })
                            .collect::<(Vec<_>, Vec<_>)>();

                        quote! {
                            #name::#variant_name(#(#field_patterns),*) => {
                                let mut dbg = f.debug_tuple(#variant_str);
                                #(#field_debugs)*
                                dbg.finish()
                            },
                        }
                    }
                    Fields::Named(fields) => {
                        let field_names: Vec<_> = fields.named
                            .iter()
                            .map(|f| &f.ident)
                            .collect();

                        quote! {
                            #name::#variant_name { #(#field_names),* } => {
                                let mut dbg = f.debug_struct(#variant_str);
                                #(dbg.field(stringify!(#field_names), &#field_names);)*
                                dbg.finish()
                            },
                        }
                    }
                }
            });

            quote! {
                impl std::fmt::Debug for #name {
                    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        match self {
                            #(#variants)*
                        }
                    }
                }
            }
        }
        _ => {
            return syn::Error
                ::new_spanned(&input, "DebugFlat can only be derived for enums")
                .to_compile_error()
                .into();
        }
    };

    TokenStream::from(debug_impl)
}

fn has_debug_skip(attrs: &[Attribute]) -> bool {
    attrs.into_iter().any(|attr| {
        let Meta::List(ref list) = attr.meta else {
            return false;
        };

        if !list.path.is_ident("debug") {
            return false;
        }

        list.tokens
            .clone()
            .into_iter()
            .any(|token| {
                let proc_macro2::TokenTree::Ident(ident) = token else {
                    return false;
                };
                ident == "skip"
            })
    })
}

fn has_debug_flatten(attrs: &[Attribute]) -> bool {
    attrs.into_iter().any(|attr| {
        let Meta::List(ref list) = attr.meta else {
            return false;
        };

        if !list.path.is_ident("debug") {
            return false;
        }

        list.tokens
            .clone()
            .into_iter()
            .any(|token| {
                let proc_macro2::TokenTree::Ident(ident) = token else {
                    return false;
                };
                ident == "flatten"
            })
    })
}
