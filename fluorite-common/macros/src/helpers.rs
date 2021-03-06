use crate::kw;
use syn::{
    braced, bracketed, parenthesized,
    parse::{Parse, ParseStream, Result},
    punctuated::Punctuated,
    spanned::Spanned,
    token, Attribute, Error, Generics, Ident, Lit, Token, Type, Visibility, WhereClause,
};

pub enum Bits {
    Single(Lit),
    Range { start: Lit, end: Lit },
    RangeInclusive { start: Lit, end: Lit },
    OffsetAndLength { start: Lit, length: Lit },
    RangeFull,
}

pub enum AccessorKind {
    None,
    Conv(Type),
    UnsafeConv(Type),
    TryConv(Type),
    Disabled,
}

pub struct Field {
    pub attrs: Vec<Attribute>,
    pub vis: Visibility,
    pub ident: Ident,
    pub ty: Type,
    pub get_ty: AccessorKind,
    pub set_ty: AccessorKind,
    pub bits: Bits,
}

pub struct Struct {
    pub outer_attrs: Vec<Attribute>,
    pub vis: Visibility,
    pub ident: Ident,
    pub storage_vis: Visibility,
    pub storage_ty: Type,
    pub generics: Generics,
    pub where_clause: Option<WhereClause>,
    pub fields: Punctuated<Field, Token![,]>,
}

impl Parse for Struct {
    fn parse(input: ParseStream) -> Result<Self> {
        let outer_attrs = input.call(Attribute::parse_outer)?;
        let vis = input.parse()?;
        input.parse::<Token![struct]>()?;
        let ident = input.parse::<Ident>()?;
        let generics = input.parse::<Generics>()?;

        let (storage_vis, storage_ty) = {
            let content;
            parenthesized!(content in input);
            (content.parse()?, content.parse()?)
        };

        let mut lookahead = input.lookahead1();
        let mut where_clause = None;
        if lookahead.peek(Token![where]) {
            where_clause = Some(input.parse()?);
            lookahead = input.lookahead1();
        };

        if where_clause.is_none() && lookahead.peek(token::Paren) {
            return Err(Error::new(input.span(), "Tuple structs are not supported"));
        } else if lookahead.peek(Token![;]) {
            return Err(Error::new(input.span(), "Empty structs are not supported"));
        } else if !lookahead.peek(token::Brace) {
            return Err(Error::new(input.span(), "Unknown struct type"));
        }

        let content;
        braced!(content in input);
        assert!(
            content.call(Attribute::parse_inner)?.is_empty(),
            "Inner attributes are not supported right now"
        );
        let fields = content.parse_terminated(|input| {
            let attrs = input.call(Attribute::parse_outer)?;
            let vis = input.parse()?;
            let ident = input.parse()?;
            input.parse::<Token![:]>()?;
            let ty = input.parse()?;
            let (get_ty, set_ty) = {
                let lookahead = input.lookahead1();
                if lookahead.peek(token::Bracket) {
                    let content;
                    bracketed!(content in input);
                    let mut get_ty = AccessorKind::None;
                    let mut set_ty = AccessorKind::None;
                    macro_rules! check_not_duplicated {
                        ($($ident: ident),*; $span: expr) => {
                            if $(!matches!(&$ident, AccessorKind::None))||* {
                                return Err(Error::new(
                                    $span,
                                    "Duplicated conversion type definition",
                                ));
                            }
                        };
                    }
                    while !content.is_empty() {
                        if let Ok(kw) = content.parse::<kw::get>() {
                            check_not_duplicated!(get_ty; kw.span);
                            get_ty = AccessorKind::Conv(content.parse()?);
                        } else if let Ok(kw) = content.parse::<kw::try_get>() {
                            check_not_duplicated!(get_ty; kw.span);
                            get_ty = AccessorKind::TryConv(content.parse()?);
                        } else if let Ok(kw) = content.parse::<kw::unsafe_get>() {
                            check_not_duplicated!(get_ty; kw.span);
                            get_ty = AccessorKind::UnsafeConv(content.parse()?);
                        } else if let Ok(kw) = content.parse::<kw::set>() {
                            check_not_duplicated!(set_ty; kw.span);
                            set_ty = AccessorKind::Conv(content.parse()?);
                        } else if let Ok(kw) = content.parse::<kw::try_set>() {
                            check_not_duplicated!(set_ty; kw.span);
                            set_ty = AccessorKind::TryConv(content.parse()?);
                        } else if let Ok(kw) = content.parse::<Token![try]>() {
                            check_not_duplicated!(get_ty, set_ty; kw.span);
                            let ty: Type = content.parse()?;
                            get_ty = AccessorKind::TryConv(ty.clone());
                            set_ty = AccessorKind::TryConv(ty);
                        } else if let Ok(kw) = content.parse::<Token![unsafe]>() {
                            check_not_duplicated!(get_ty, set_ty; kw.span);
                            let ty: Type = content.parse()?;
                            get_ty = AccessorKind::UnsafeConv(ty.clone());
                            set_ty = AccessorKind::UnsafeConv(ty);
                        } else if let Ok(kw) = content.parse::<kw::read_only>() {
                            if matches!(&get_ty, AccessorKind::Disabled) {
                                return Err(Error::new(
                                    kw.span,
                                    "Duplicated read_only and write_only specifiers",
                                ));
                            }
                            set_ty = AccessorKind::Disabled;
                        } else if let Ok(kw) = content.parse::<kw::write_only>() {
                            if matches!(&set_ty, AccessorKind::Disabled) {
                                return Err(Error::new(
                                    kw.span,
                                    "Duplicated read_only and write_only specifiers",
                                ));
                            }
                            get_ty = AccessorKind::Disabled;
                        } else {
                            let ty: Type = content.parse()?;
                            check_not_duplicated!(get_ty, set_ty; ty.span());
                            get_ty = AccessorKind::TryConv(ty.clone());
                            set_ty = AccessorKind::TryConv(ty);
                        }
                        let _ = content.parse::<Token![,]>();
                    }
                    (get_ty, set_ty)
                } else {
                    (AccessorKind::None, AccessorKind::None)
                }
            };
            input.parse::<Token![@]>()?;
            Ok(Field {
                attrs,
                vis,
                ident,
                ty,
                get_ty,
                set_ty,
                bits: {
                    if input.parse::<Token![..]>().is_ok() {
                        Bits::RangeFull
                    } else {
                        let start = input.parse()?;
                        let lookahead = input.lookahead1();
                        if lookahead.peek(Token![..=]) {
                            input.parse::<Token![..=]>()?;
                            let end = input.parse()?;
                            Bits::RangeInclusive { start, end }
                        } else if lookahead.peek(Token![..]) {
                            input.parse::<Token![..]>()?;
                            let end = input.parse()?;
                            Bits::Range { start, end }
                        } else if lookahead.peek(Token![;]) {
                            input.parse::<Token![;]>()?;
                            let length = input.parse()?;
                            Bits::OffsetAndLength { start, length }
                        } else if lookahead.peek(Token![,]) || input.is_empty() {
                            Bits::Single(start)
                        } else {
                            return Err(lookahead.error());
                        }
                    }
                },
            })
        })?;

        Ok(Self {
            outer_attrs,
            vis,
            ident,
            storage_vis,
            storage_ty,
            generics,
            where_clause,
            fields,
        })
    }
}
