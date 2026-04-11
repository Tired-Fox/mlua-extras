use proc_macro2::TokenStream;
use quote::quote;
use serde::Deserialize;
use syn::{
    FnArg, ImplItem, ImplItemFn, ItemImpl, Lit, Meta, Pat, ReturnType, Type,
};

#[derive(Deserialize, Default)]
struct MethodAttr {
    #[serde(default)]
    rename: Option<String>,
}

#[derive(Clone)]
enum ReceiverKind {
    Ref,
    RefMut,
    None,
}

enum MethodKind {
    Regular,
    Metamethod(TokenStream),
}

struct MethodInfo {
    fn_name: syn::Ident,
    lua_name: TokenStream,
    receiver: ReceiverKind,
    is_async: bool,
    has_lua_param: bool,
    params: Vec<(syn::Ident, syn::Type)>,
    is_fallible: bool,
    has_return: bool,
    kind: MethodKind,
    doc: Option<String>,
}

fn extract_doc_comment(attrs: &[syn::Attribute]) -> Option<String> {
    let docs: Vec<String> = attrs
        .iter()
        .filter_map(|attr| {
            if !attr.path().is_ident("doc") {
                return None;
            }
            if let Meta::NameValue(nv) = &attr.meta {
                if let syn::Expr::Lit(expr_lit) = &nv.value {
                    if let Lit::Str(s) = &expr_lit.lit {
                        return Some(s.value().trim().to_string());
                    }
                }
            }
            None
        })
        .collect();

    if docs.is_empty() {
        None
    } else {
        Some(docs.join("\n"))
    }
}

fn is_method_attr(attr: &syn::Attribute) -> bool {
    attr.path().is_ident("method")
}

fn is_metamethod_attr(attr: &syn::Attribute) -> bool {
    attr.path().is_ident("metamethod")
}

fn parse_method_attr(attr: &syn::Attribute) -> MethodAttr {
    match &attr.meta {
        Meta::Path(_) => MethodAttr::default(),
        Meta::List(list) => match serde_tokenstream::from_tokenstream(&list.tokens) {
            Ok(v) => v,
            Err(e) => {
                proc_macro_error::abort!(list.tokens, "invalid method attribute: {}", e);
            }
        },
        _ => {
            proc_macro_error::abort!(attr, "expected #[method] or #[method(...)]");
        }
    }
}

fn parse_metamethod_arg(attr: &syn::Attribute) -> TokenStream {
    match &attr.meta {
        Meta::List(list) => {
            let tokens = &list.tokens;
            // Try to parse as a single ident or string literal
            let token_str = tokens.to_string();
            let trimmed = token_str.trim();

            // Check if it's a string literal
            if trimmed.starts_with('"') && trimmed.ends_with('"') {
                // It's a string literal, emit as-is
                tokens.clone()
            } else {
                // It's an identifier, emit as-is (will resolve via wildcard import)
                tokens.clone()
            }
        }
        _ => {
            proc_macro_error::abort!(attr, "expected #[metamethod(Variant)] or #[metamethod(\"name\")]");
        }
    }
}

fn classify_receiver(method: &ImplItemFn) -> ReceiverKind {
    if let Some(first_arg) = method.sig.inputs.first() {
        match first_arg {
            FnArg::Receiver(recv) => {
                if recv.reference.is_some() {
                    if recv.mutability.is_some() {
                        ReceiverKind::RefMut
                    } else {
                        ReceiverKind::Ref
                    }
                } else {
                    // Value receiver — check for Arc<Self> pattern
                    // For now treat as Ref (the common case with Arc wrapping)
                    ReceiverKind::Ref
                }
            }
            FnArg::Typed(pat_type) => {
                // Check for `self: &Arc<Self>` pattern
                if let Pat::Ident(pat_ident) = &*pat_type.pat {
                    if pat_ident.ident == "self" {
                        // Treat typed self as &self for now
                        return ReceiverKind::Ref;
                    }
                }
                ReceiverKind::None
            }
        }
    } else {
        ReceiverKind::None
    }
}

fn is_result_type(ty: &Type) -> bool {
    match ty {
        Type::Path(type_path) => {
            if let Some(last_seg) = type_path.path.segments.last() {
                last_seg.ident == "Result"
            } else {
                false
            }
        }
        _ => false,
    }
}

fn analyze_method(method: &ImplItemFn, kind: MethodKind) -> MethodInfo {
    let fn_name = method.sig.ident.clone();
    let is_async = method.sig.asyncness.is_some();
    let receiver = classify_receiver(method);
    let doc = extract_doc_comment(&method.attrs);

    // Check for async metamethod conflict
    if is_async {
        if let MethodKind::Metamethod(_) = &kind {
            proc_macro_error::abort!(
                method.sig.asyncness,
                "async metamethods are not supported by mlua"
            );
        }
    }

    // Collect non-self parameters
    let mut params_iter = method.sig.inputs.iter();

    // Skip the receiver if present
    match &receiver {
        ReceiverKind::Ref | ReceiverKind::RefMut => {
            params_iter.next(); // skip self
        }
        ReceiverKind::None => {
            // Check if the first arg is a typed `self` pattern
            if let Some(FnArg::Typed(pat_type)) = method.sig.inputs.first() {
                if let Pat::Ident(pat_ident) = &*pat_type.pat {
                    if pat_ident.ident == "self" {
                        params_iter.next();
                    }
                }
            }
        }
    }

    let remaining_params: Vec<_> = params_iter
        .filter_map(|arg| {
            if let FnArg::Typed(pat_type) = arg {
                if let Pat::Ident(pat_ident) = &*pat_type.pat {
                    return Some((pat_ident.ident.clone(), (*pat_type.ty).clone()));
                }
            }
            None
        })
        .collect();

    // Detect lua parameter (first param named "lua")
    let has_lua_param = remaining_params
        .first()
        .map(|(name, _)| name == "lua")
        .unwrap_or(false);

    let params: Vec<(syn::Ident, syn::Type)> = if has_lua_param {
        remaining_params.into_iter().skip(1).collect()
    } else {
        remaining_params
    };

    // Analyze return type
    let (is_fallible, has_return) = match &method.sig.output {
        ReturnType::Default => (false, false),
        ReturnType::Type(_, ty) => {
            if is_result_type(ty) {
                (true, true)
            } else {
                // Check if it's () type
                let is_unit = matches!(&**ty, Type::Tuple(t) if t.elems.is_empty());
                (false, !is_unit)
            }
        }
    };

    // Determine lua name
    let lua_name = match &kind {
        MethodKind::Metamethod(arg) => arg.clone(),
        MethodKind::Regular => {
            let name_str = fn_name.to_string();
            quote! { #name_str }
        }
    };

    MethodInfo {
        fn_name,
        lua_name,
        receiver,
        is_async,
        has_lua_param,
        params,
        is_fallible,
        has_return,
        kind,
        doc,
    }
}

fn generate_registration(info: &MethodInfo, self_ty: &Type) -> TokenStream {
    let fn_name = &info.fn_name;
    let lua_name = &info.lua_name;

    let param_names: Vec<_> = info.params.iter().map(|(name, _)| name).collect();
    let param_types: Vec<_> = info.params.iter().map(|(_, ty)| ty).collect();

    // Build the parameter destructuring for the closure
    let params_destructure = if param_names.is_empty() {
        quote! { _: () }
    } else {
        quote! { (#(#param_names,)*): (#(#param_types,)*) }
    };

    // Build the method call arguments
    let call_args = if info.has_lua_param {
        let args = &param_names;
        quote! { lua, #(#args,)* }
    } else {
        let args = &param_names;
        quote! { #(#args,)* }
    };

    let lua_ident = if info.has_lua_param {
        quote! { lua }
    } else {
        quote! { _lua }
    };

    // Build the method call and return wrapping
    let build_call_and_return = |call: TokenStream| -> TokenStream {
        if info.is_fallible {
            quote! { #call.map_err(|e| e.into()) }
        } else if info.has_return {
            quote! {
                let result = #call;
                Ok(result)
            }
        } else {
            quote! {
                #call;
                Ok(())
            }
        }
    };

    let is_meta = matches!(info.kind, MethodKind::Metamethod(_));

    // For metamethods, wrap the name in a block with wildcard import
    let wrap_meta_name = |_name: &TokenStream, body: TokenStream| -> TokenStream {
        if is_meta {
            quote! {
                {
                    use mlua_extras::mlua::MetaMethod::*;
                    #body
                }
            }
        } else {
            body
        }
    };

    let doc_stmt = info.doc.as_ref().map(|doc| {
        quote! { methods.document(#doc); }
    });

    let param_stmts: Vec<_> = info.params.iter().map(|(name, _)| {
        let name_str = name.to_string();
        quote! { methods.param(#name_str, ""); }
    }).collect();

    if info.is_async {
        // Async methods
        let call = match &info.receiver {
            ReceiverKind::Ref => {
                let body = build_call_and_return(quote! { this.#fn_name(#call_args).await });
                let reg = quote! {
                    #doc_stmt
                    #(#param_stmts)*
                    methods.add_async_method(#lua_name, |#lua_ident, this, #params_destructure| async move {
                        #body
                    });
                };
                wrap_meta_name(lua_name, reg)
            }
            ReceiverKind::RefMut => {
                let body = build_call_and_return(quote! { this.#fn_name(#call_args).await });
                let reg = quote! {
                    #doc_stmt
                    #(#param_stmts)*
                    methods.add_async_method_mut(#lua_name, |#lua_ident, this, #params_destructure| async move {
                        #body
                    });
                };
                wrap_meta_name(lua_name, reg)
            }
            ReceiverKind::None => {
                let body =
                    build_call_and_return(quote! { #self_ty::#fn_name(#call_args).await });
                let reg = quote! {
                    #doc_stmt
                    #(#param_stmts)*
                    methods.add_async_function(#lua_name, |#lua_ident, #params_destructure| async move {
                        #body
                    });
                };
                wrap_meta_name(lua_name, reg)
            }
        };
        call
    } else {
        // Sync methods
        match (&info.receiver, is_meta) {
            (ReceiverKind::Ref, false) => {
                let body = build_call_and_return(quote! { this.#fn_name(#call_args) });
                quote! {
                    #doc_stmt
                    #(#param_stmts)*
                    methods.add_method(#lua_name, |#lua_ident, this, #params_destructure| {
                        #body
                    });
                }
            }
            (ReceiverKind::Ref, true) => {
                let body = build_call_and_return(quote! { this.#fn_name(#call_args) });
                quote! {
                    {
                        use mlua_extras::mlua::MetaMethod::*;
                        #doc_stmt
                        #(#param_stmts)*
                        methods.add_meta_method(#lua_name, |#lua_ident, this, #params_destructure| {
                            #body
                        });
                    }
                }
            }
            (ReceiverKind::RefMut, false) => {
                let body = build_call_and_return(quote! { this.#fn_name(#call_args) });
                quote! {
                    #doc_stmt
                    #(#param_stmts)*
                    methods.add_method_mut(#lua_name, |#lua_ident, this, #params_destructure| {
                        #body
                    });
                }
            }
            (ReceiverKind::RefMut, true) => {
                let body = build_call_and_return(quote! { this.#fn_name(#call_args) });
                quote! {
                    {
                        use mlua_extras::mlua::MetaMethod::*;
                        #doc_stmt
                        #(#param_stmts)*
                        methods.add_meta_method_mut(#lua_name, |#lua_ident, this, #params_destructure| {
                            #body
                        });
                    }
                }
            }
            (ReceiverKind::None, false) => {
                let body =
                    build_call_and_return(quote! { #self_ty::#fn_name(#call_args) });
                quote! {
                    #doc_stmt
                    #(#param_stmts)*
                    methods.add_function(#lua_name, |#lua_ident, #params_destructure| {
                        #body
                    });
                }
            }
            (ReceiverKind::None, true) => {
                let body =
                    build_call_and_return(quote! { #self_ty::#fn_name(#call_args) });
                quote! {
                    {
                        use mlua_extras::mlua::MetaMethod::*;
                        #doc_stmt
                        #(#param_stmts)*
                        methods.add_meta_function(#lua_name, |#lua_ident, #params_destructure| {
                            #body
                        });
                    }
                }
            }
        }
    }
}

pub fn methods_impl(item: ItemImpl) -> TokenStream {
    let self_ty = &item.self_ty;

    let mut method_infos = Vec::new();
    let mut cleaned_items = Vec::new();

    for impl_item in &item.items {
        match impl_item {
            ImplItem::Fn(method) => {
                let method_attr = method.attrs.iter().find(|a| is_method_attr(a));
                let metamethod_attr = method.attrs.iter().find(|a| is_metamethod_attr(a));

                if method_attr.is_some() && metamethod_attr.is_some() {
                    proc_macro_error::abort!(
                        method.sig.ident.span(),
                        "cannot have both #[method] and #[metamethod] on the same function"
                    );
                }

                if let Some(attr) = method_attr {
                    let mattr = parse_method_attr(attr);
                    let kind = MethodKind::Regular;
                    let mut info = analyze_method(method, kind);

                    // Apply rename if specified
                    if let Some(rename) = mattr.rename {
                        info.lua_name = quote! { #rename };
                    }

                    method_infos.push(info);

                    // Strip the #[method] attribute from the cleaned item
                    let mut cleaned = method.clone();
                    cleaned
                        .attrs
                        .retain(|a| !is_method_attr(a));
                    cleaned_items.push(ImplItem::Fn(cleaned));
                } else if let Some(attr) = metamethod_attr {
                    let meta_arg = parse_metamethod_arg(attr);
                    let kind = MethodKind::Metamethod(meta_arg);
                    let info = analyze_method(method, kind);
                    method_infos.push(info);

                    // Strip the #[metamethod] attribute from the cleaned item
                    let mut cleaned = method.clone();
                    cleaned
                        .attrs
                        .retain(|a| !is_metamethod_attr(a));
                    cleaned_items.push(ImplItem::Fn(cleaned));
                } else {
                    // Not annotated — keep as-is
                    cleaned_items.push(impl_item.clone());
                }
            }
            _ => {
                cleaned_items.push(impl_item.clone());
            }
        }
    }

    let registrations: Vec<_> = method_infos
        .iter()
        .map(|info| generate_registration(info, self_ty))
        .collect();

    // Reconstruct the cleaned impl block
    let attrs = &item.attrs;
    let unsafety = &item.unsafety;
    let impl_token = &item.impl_token;
    let generics = &item.generics;

    quote! {
        #(#attrs)*
        #unsafety #impl_token #generics #self_ty {
            #(#cleaned_items)*
        }

        impl #generics #self_ty {
            #[doc(hidden)]
            fn __auto_add_methods<M: mlua_extras::typed::TypedDataMethods<Self>>(methods: &mut M) {
                #(#registrations)*
            }
        }
    }
}
