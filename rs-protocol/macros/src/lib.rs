use proc_macro::TokenStream;
use quote::quote;
use syn::parse::Parser;
use syn::{DeriveInput, Expr, parse_macro_input};

#[proc_macro_attribute]
pub fn client_prot(args: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let args: syn::punctuated::Punctuated<Expr, syn::Token![,]> =
        syn::punctuated::Punctuated::parse_terminated
            .parse(args)
            .unwrap();

    let mut iter = args.iter();

    let frame = iter.next().expect("expected frame kind");
    let frame_expr = quote! { ClientProtFrame::#frame };

    let category_expr = match iter.next().expect("expected category") {
        Expr::Path(p) => {
            let ident = p.path.get_ident().unwrap();
            quote! { ClientProtCategory::#ident }
        }
        _ => panic!("expected identifier for category"),
    };

    quote! {
        #input

        impl ClientProtMessageInfo for #name {
            const FRAME: ClientProtFrame = #frame_expr;
            const CATEGORY: ClientProtCategory = #category_expr;
        }
    }
    .into()
}

#[proc_macro_attribute]
pub fn server_prot(args: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let args: syn::punctuated::Punctuated<Expr, syn::Token![,]> =
        syn::punctuated::Punctuated::parse_terminated
            .parse(args)
            .unwrap();

    let mut iter = args.iter();

    let prot_expr = match iter.next().expect("expected prot variant") {
        Expr::Path(p) => {
            let ident = p.path.get_ident().unwrap();
            quote! { ServerProt::#ident }
        }
        _ => panic!("expected identifier for prot"),
    };

    let priority_expr = match iter.next().expect("expected priority") {
        Expr::Path(p) => {
            let ident = p.path.get_ident().unwrap();
            quote! { ServerProtPriority::#ident }
        }
        _ => panic!("expected identifier for priority"),
    };

    let frame_expr = match iter.next().expect("expected frame kind") {
        Expr::Path(p) => {
            let ident = p.path.get_ident().unwrap();
            quote! { PacketFrame::#ident }
        }
        _ => panic!("expected frame kind"),
    };

    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    quote! {
        #input

        impl #impl_generics ServerProtMessageInfo for #name #ty_generics #where_clause {
            const PROT: ServerProt = #prot_expr;
            const PRIORITY: ServerProtPriority = #priority_expr;
            const FRAME: PacketFrame = #frame_expr;
        }
    }
    .into()
}
