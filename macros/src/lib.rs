extern crate proc_macro;
extern crate proc_macro2;
extern crate quote;
extern crate syn;

use proc_macro as pm;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::parse::{Parse, Parser};
use syn::{Attribute, DataEnum, DeriveInput, Meta, Token};

struct EnabledNodes {
    analyzer: bool,
    generator: bool,
    pretty: bool,
    spanned: bool,
}

impl EnabledNodes {
    const ALL: Self = Self {
        analyzer: true,
        generator: true,
        pretty: true,
        spanned: true,
    };

    fn has_any_enabled(&self) -> bool {
        self.analyzer || self.generator || self.pretty || self.spanned
    }
}

#[proc_macro_derive(Node, attributes(node, spanned))]
pub fn node_derive(input: pm::TokenStream) -> pm::TokenStream {
    let DeriveInput {
        attrs,
        ident: struct_id,
        data,
        ..
    } = syn::parse_macro_input!(input as DeriveInput);

    let Some(enabled_nodes) = get_attr(attrs, "node") else {
        return pm::TokenStream::new();
    };
    let enabled_nodes = match enabled_nodes.meta {
        Meta::List(l) => match parse_enabled_nodes.parse2(l.tokens) {
            Ok(v) => v,
            Err(err) => return err.to_compile_error().into(),
        },
        Meta::Path(_) => EnabledNodes::ALL,
        _ => return pm::TokenStream::new(),
    };

    if !enabled_nodes.has_any_enabled() {
        return pm::TokenStream::new();
    }

    match data {
        syn::Data::Struct(_) => pm::TokenStream::new(),
        syn::Data::Enum(data) => node_derive_enum(struct_id, enabled_nodes, data),
        _ => pm::TokenStream::new(),
    }
}

fn node_derive_enum(
    struct_id: syn::Ident,
    enabled_nodes: EnabledNodes,
    data: DataEnum,
) -> pm::TokenStream {
    let spanned = if enabled_nodes.spanned {
        let variants = data.variants.iter().map(|v| {
            let name = &v.ident;
            match &v.fields {
                syn::Fields::Named(_) => todo!("Forward to struct"),
                syn::Fields::Unit => quote! {Self::#name => Span::default()},
                syn::Fields::Unnamed(fields) if fields.unnamed.is_empty() => {
                    quote! {Self::#name() => Span::default()}
                }
                syn::Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                    let ty = fields.unnamed.first().unwrap();
                    quote! {Self::#name(v1) => <#ty as IntoSpanned>::span(v1)}
                }
                syn::Fields::Unnamed(fields) => {
                    let decl = fields
                        .unnamed
                        .iter()
                        .enumerate()
                        .map(|(c, _)| format_ident!("_arg_{c}"));

                    let last_arg = format_ident!("_arg_{}", fields.unnamed.len() - 1);

                    let first_ty = &fields.unnamed.first().unwrap().ty;
                    let last_ty = &fields.unnamed.last().unwrap().ty;

                    quote! {Self::#name(#(#decl),*) => <#first_ty as IntoSpanned>::span(_arg_0).merge(<#last_ty as IntoSpanned>::span(#last_arg))}
                }
            }
        });

        quote! {const _: () = {
            use ::amrisk::parser::*;
            impl IntoSpanned for #struct_id {
                fn span(&self) -> Span {
                    match self {
                        #(#variants),*
                    }
                }
            }
        };}
    } else {
        TokenStream::new()
    };

    let pretty = if enabled_nodes.pretty {
        let variants = data.variants.iter().map(|v| {
            let name = &v.ident;
            match &v.fields {
                syn::Fields::Named(_) => quote! {_ => compile_error!("Named variants are not supported by pretty")},
                syn::Fields::Unit => quote! {Self::#name => Ok(())},
                syn::Fields::Unnamed(fields) if fields.unnamed.is_empty() => quote! {Self::#name() => Ok(())},
                syn::Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                    let ty = fields.unnamed.first().unwrap();
                    quote! {Self::#name(v1) => <#ty as PrettyPrint>::pretty_print(v1, f)}
                }
                syn::Fields::Unnamed(_) => quote! {_ => compile_error!("Multi-unamed variants are not supported by pretty")}
            }
        });

        quote! {const _: () = {
            use ::amrisk::pretty::*;
            impl PrettyPrint for #struct_id {
                fn pretty_print(&self, f: &mut PrettyFormatter) -> fmt::Result {
                    match self {
                        #(#variants),*
                    }
                }
            }
        };}
    } else {
        TokenStream::new()
    };

    let analyzer = if enabled_nodes.analyzer {
        let variants = data.variants.iter().map(|v| {
            let name = &v.ident;
            match &v.fields {
                syn::Fields::Named(_) => quote! {_ => compile_error!("Named variants are not supported by analyzer")},
                syn::Fields::Unit => quote! {Self::#name => AnalyzeResult::Continue(())},
                syn::Fields::Unnamed(fields) if fields.unnamed.is_empty() => quote! {Self::#name() => AnalyzeResult::Continue(())},
                syn::Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                    let ty = fields.unnamed.first().unwrap();
                    quote! {Self::#name(v1) => <#ty as Analyze>::analyze(v1, s)}
                }
                syn::Fields::Unnamed(_) => quote! {_ => compile_error!("Multi-unamed variants are not supported by analyzer")}
            }
        });

        quote! {const _: () = {
            use ::amrisk::analysis::*;
            impl Analyze for #struct_id {
                fn analyze(&mut self, s: &mut AnalyzeSummary) -> AnalyzeResult {
                    match self {
                        #(#variants),*
                    }
                }
            }
        };}
    } else {
        TokenStream::new()
    };

    let generator = if enabled_nodes.generator {
        let variants = data.variants.iter().map(|v| {
            let name = &v.ident;
            match &v.fields {
                syn::Fields::Named(_) => quote! {_ => compile_error!("Named variants are not supported by generator")},
                syn::Fields::Unit => quote! {Self::#name => ()},
                syn::Fields::Unnamed(fields) if fields.unnamed.is_empty() => quote! {Self::#name() => ()},
                syn::Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                    let ty = fields.unnamed.first().unwrap();
                    quote! {Self::#name(v1) => <#ty as Generate>::generate(v1, b)}
                }
                syn::Fields::Unnamed(_) => quote! {_ => compile_error!("Multi-unamed variants are not supported by generator")}
            }
        });

        quote! {const _: () = {
            use ::amrisk::generator::*;
            impl Generate for #struct_id {
                fn generate(&self, b: &mut GenerateBuf) {
                    match self {
                        #(#variants),*
                    }
                }
            }
        };}
    } else {
        TokenStream::new()
    };

    quote! {
        #analyzer
        #generator
        #pretty
        #spanned
    }
    .into()
}

fn get_attr(attrs: Vec<Attribute>, name: &'static str) -> Option<Attribute> {
    attrs
        .into_iter()
        .find(|a| a.path().get_ident().is_some_and(|i| *i == name))
}

fn parse_enabled_nodes(input: syn::parse::ParseStream) -> syn::Result<EnabledNodes> {
    let list = input.parse_terminated(syn::Ident::parse, Token![,])?;

    let mut analyzer = false;
    let mut generator = false;
    let mut pretty = false;
    let mut spanned = false;

    for ident in list {
        match &*ident.to_string() {
            "analyzer" => analyzer = true,
            "generator" => generator = true,
            "pretty" => pretty = true,
            "spanned" => spanned = true,
            _ => return syn::Result::Err(syn::Error::new(ident.span(), "Unrecognized node")),
        }
    }

    Ok(EnabledNodes {
        analyzer,
        generator,
        pretty,
        spanned,
    })
}
