use crate::helpers::{AccessorKind, Bits};
use helpers::Field;
use proc_macro::TokenStream;
use quote::__private::Span;
use quote::{format_ident, quote, quote_spanned};
use syn::Ident;

mod helpers;
mod kw;

#[proc_macro]
pub fn bitfield(input: TokenStream) -> TokenStream {
    emit_bitfield(input, false)
}

#[proc_macro]
pub fn bitfield_debug(input: TokenStream) -> TokenStream {
    emit_bitfield(input, true)
}

fn emit_bitfield(input: TokenStream, debug: bool) -> TokenStream {
    let crate::helpers::Struct {
        outer_attrs,
        vis,
        ident,
        storage_vis,
        storage_ty,
        generics,
        where_clause,
        fields,
    } = match syn::parse(input) {
        Ok(res) => res,
        Err(err) => return err.into_compile_error().into(),
    };

    let const_fields = fields
        .iter()
        .map(|Field { ident, bits, .. }| {
            if let Bits::Single(bit) = bits {
                let name = ident.to_string().to_uppercase();
                let name = Ident::new(&name, Span::call_site());
                quote! { pub const #name: #storage_ty = 1 << #bit; }
            } else {
                quote! {}
            }
        })
        .collect::<Vec<_>>();

    let field_fns = fields.iter().map(
        |Field {
             attrs,
             vis,
             ident,
             ty,
             get_ty,
             set_ty,
             bits,
         }| {
            let storage_ty_bits = quote! { (::core::mem::size_of::<#storage_ty>() << 3) };
            let ty_bits = quote! { (::core::mem::size_of::<#ty>() << 3) };
            let set_fn_ident = format_ident!("set_{}", ident);
            let with_fn_ident = format_ident!("with_{}", ident);
            let (start, end) = match bits {
                Bits::Single(bit) => (quote! { #bit }, None),
                Bits::Range { start, end } => (quote! { #start }, Some(quote! { #end })),
                Bits::RangeInclusive { start, end } => (
                    quote! { #start },
                    Some(quote! { {#end + 1} }),
                ),
                Bits::OffsetAndLength { start, length } => (
                    quote! { #start },
                    Some(quote! { {#start + #length} }),
                ),
                Bits::RangeFull => (
                    quote! { 0 },
                    Some(quote!{ { ::core::mem::size_of::<#ty>() << 3 } }),
                ),
            };

            let getter = if !matches!(&get_ty, AccessorKind::Disabled) {
                let (calc_get_result, get_output_ty) = match get_ty {
                    AccessorKind::None => (quote! { raw_result }, quote! { #ty }),
                    AccessorKind::Disabled => unreachable!(),
                    AccessorKind::Conv(get_ty) => {
                        (
                            quote! { <#get_ty as ::core::convert::From<#ty>>::from(raw_result) },
                            quote! { #get_ty },
                        )
                    }
                    AccessorKind::UnsafeConv(get_ty) => {
                        (
                            quote! { unsafe {
                                <#get_ty as ::fluorite_common::traits::UnsafeFrom<#ty>>::from(
                                    raw_result,
                                )
                            } },
                            quote! { #get_ty },
                        )
                    }
                    AccessorKind::TryConv(get_ty) => (
                        quote! { #get_ty::try_from(raw_result) },
                        quote! {
                            Result<
                                #get_ty,
                                <#get_ty as ::core::convert::TryFrom<#ty>>::Error,
                            >
                        },
                    ),
                };
                let get_value = if let Some(end) = &end {
                    quote_spanned! {
                        ident.span() =>
                        ::fluorite_common::static_assertions::const_assert!(
                            #end > #start
                        );
                        ::fluorite_common::static_assertions::const_assert!(
                            #start < #storage_ty_bits && #end <= #storage_ty_bits
                        );
                        ::fluorite_common::static_assertions::const_assert!(
                            #end - #start <= #ty_bits
                        );
                        let raw_result = <#storage_ty as ::fluorite_common::BitRange<#ty>>
                            ::bit_range::<#start, #end>(self.0);
                    }
                } else {
                    quote_spanned! {
                        ident.span() =>
                        ::fluorite_common::static_assertions::const_assert!(#start < #storage_ty_bits);
                        let raw_result = <#storage_ty as ::fluorite_common::Bit>
                            ::bit::<#start>(self.0);
                    }
                };
                quote! {
                    #(#attrs)*
                    #[inline]
                    #[allow(clippy::identity_op)]
                    #vis fn #ident(&self) -> #get_output_ty {
                        #get_value
                        #calc_get_result
                    }
                }
            } else {
                quote! {}
            };

            let setters = if !matches!(&set_ty, AccessorKind::Disabled) {
                let (
                    calc_set_with_value,
                    set_with_input_ty,
                    set_ok,
                    set_output_ty,
                    with_ok,
                    with_output_ty,
                ) = match set_ty {
                    AccessorKind::None => (
                        quote! { value },
                        ty,
                        quote! {},
                        quote! { () },
                        quote! { raw_result },
                        quote! { Self },
                    ),
                    AccessorKind::Disabled => unreachable!(),
                    AccessorKind::Conv(set_ty) | AccessorKind::UnsafeConv(set_ty) => (
                        quote! { <#set_ty as ::core::convert::Into<#ty>>::into(value) },
                        set_ty,
                        quote! {},
                        quote! { () },
                        quote! { raw_result },
                        quote! { Self },
                    ),
                    AccessorKind::TryConv(set_ty) => (
                        quote! { #set_ty::try_into(value)? },
                        set_ty,
                        quote! { Ok(()) },
                        quote! { Result<(), <#set_ty as ::core::convert::TryInto<#ty>>::Error> },
                        quote! { Ok(raw_result) },
                        quote! { Result<Self, <#set_ty as ::core::convert::TryInto<#ty>>::Error> },
                    ),
                };
                let with_value = if let Some(end) = &end {
                    quote_spanned! {
                        ident.span() =>
                        <#storage_ty as ::fluorite_common::BitRange<#ty>>
                            ::set_bit_range::<#start, #end>(self.0, #calc_set_with_value)
                    }
                } else {
                    quote_spanned! {
                        ident.span() =>
                        <#storage_ty as ::fluorite_common::Bit>::set_bit::<#start>(
                            self.0,
                            #calc_set_with_value,
                        )
                    }
                };
                quote! {
                    #(#attrs)*
                    #[inline]
                    #[must_use]
                    #[allow(clippy::identity_op)]
                    #vis fn #with_fn_ident(self, value: #set_with_input_ty) -> #with_output_ty {
                        let raw_result = Self(#with_value);
                        #with_ok
                    }

                    #(#attrs)*
                    #[inline]
                    #[allow(clippy::identity_op)]
                    #vis fn #set_fn_ident(&mut self, value: #set_with_input_ty) -> #set_output_ty {
                        self.0 = #with_value;
                        #set_ok
                    }
                }
            } else {
                quote! {}
            };

            quote! {
                #getter
                #setters
            }
        },
    ).collect::<Vec<_>>();

    let debug_impl = if debug {
        let field_idents = fields.iter().map(|field| &field.ident);
        Some(quote! {
            impl #generics ::core::fmt::Debug for #ident #where_clause {
                fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                    f.debug_struct(stringify!(#ident))
                        .field("0", &self.0)
                        #(.field(stringify!(#field_idents), &self.#field_idents()))*
                        .finish()
                }
            }
        })
    } else {
        None
    };

    (quote! {
        #( #outer_attrs )*
        #vis struct #ident #generics(#storage_vis #storage_ty) #where_clause;

        impl #generics #ident #where_clause {
            #( #const_fields )*
            #( #field_fns )*
        }

        #debug_impl
    })
    .into()
}
