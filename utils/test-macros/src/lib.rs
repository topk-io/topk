use proc_macro::TokenStream;

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::spanned::Spanned;
use syn::{parse_macro_input, Attribute, FnArg, ItemFn, Pat, PatType, Type};

// Bridge between `rstest` and `test-context`, which cannot be stacked because both
// are attribute macros that each try to own the test function (its argument list and
// the `#[test]`/`#[tokio::test]` harness). This macro lets `rstest` be the sole harness
// (it is the only one that can fan out one fn into N cases) and re-implements what
// `#[test_context]` does — setup, then run the body, then async teardown that survives
// a panicking `assert!` (via `catch_unwind` + `resume_unwind`).
//
// The first parameter is the context (a `&Ctx` or `&mut Ctx`); the rest are handed to
// `rstest` untouched, so `#[case]`, `#[values]`, and fixtures all work as usual.
//
//   #[rstest_ctx(MyCtx)]
//   #[case(1, "a")]
//   #[case(2, "b")]
//   async fn t(ctx: &mut MyCtx, #[case] n: u32, #[case] s: &str) { ... }
//
// `#[rstest]` and `#[tokio::test]` are added automatically unless you write them
// yourself (write your own to pass options, e.g. `#[tokio::test(flavor = "multi_thread")]`).
#[proc_macro_attribute]
pub fn rstest_ctx(attr: TokenStream, item: TokenStream) -> TokenStream {
    let ctx_ty = parse_macro_input!(attr as Type);
    let func = parse_macro_input!(item as ItemFn);

    match expand(ctx_ty, func) {
        Ok(ts) => ts.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

fn expand(ctx_ty: Type, func: ItemFn) -> syn::Result<TokenStream2> {
    let ItemFn {
        attrs,
        vis,
        sig,
        block,
    } = func;

    if sig.asyncness.is_none() {
        return Err(syn::Error::new(
            sig.span(),
            "#[rstest_ctx] requires an `async fn`",
        ));
    }

    // The context is the first argument. Split it off; everything after it belongs to rstest.
    let mut inputs = sig.inputs.iter().cloned();
    let ctx_arg = match inputs.next() {
        Some(FnArg::Typed(pt)) => pt,
        Some(FnArg::Receiver(r)) => {
            return Err(syn::Error::new(
                r.span(),
                "#[rstest_ctx] cannot be used on methods that take `self`",
            ));
        }
        None => {
            return Err(syn::Error::new(
                sig.ident.span(),
                "#[rstest_ctx] requires a context parameter as the first argument",
            ));
        }
    };

    let ctx_is_mut = match &*ctx_arg.ty {
        Type::Reference(r) => r.mutability.is_some(),
        other => {
            return Err(syn::Error::new(
                other.span(),
                "the context parameter must be a reference (`&Ctx` or `&mut Ctx`)",
            ));
        }
    };

    let remaining: Vec<PatType> = inputs
        .map(|arg| match arg {
            FnArg::Typed(pt) => Ok(pt),
            FnArg::Receiver(r) => Err(syn::Error::new(r.span(), "unexpected `self` parameter")),
        })
        .collect::<syn::Result<_>>()?;

    // Names to forward from the outer (rstest-owned) fn into the inner body fn.
    let forward: Vec<_> = remaining
        .iter()
        .map(|pt| match &*pt.pat {
            Pat::Ident(id) => Ok(id.ident.clone()),
            other => Err(syn::Error::new(
                other.span(),
                "#[rstest_ctx] parameters must be simple identifiers",
            )),
        })
        .collect::<syn::Result<_>>()?;

    // Outer signature keeps the remaining params *with* their rstest attributes
    // (#[case], #[values], fixtures); the context param is dropped.
    let mut outer_sig = sig.clone();
    outer_sig.inputs = remaining.iter().cloned().map(FnArg::Typed).collect();

    // Inner body fn takes the full original argument list, but with attributes stripped
    // (helper attrs like #[case] are only legal on an rstest-driven fn).
    let mut inner_sig = sig.clone();
    inner_sig.ident = syn::Ident::new("__rstest_ctx_body", sig.ident.span());
    let mut ctx_arg_plain = ctx_arg.clone();
    ctx_arg_plain.attrs.clear();
    inner_sig.inputs = std::iter::once(FnArg::Typed(ctx_arg_plain))
        .chain(remaining.iter().cloned().map(|mut pt| {
            pt.attrs.clear();
            FnArg::Typed(pt)
        }))
        .collect();

    // Ensure exactly one #[rstest] (outermost, so it fans out first) and one
    // #[tokio::test] (innermost, applied to each generated case).
    let rstest_attr = if attrs.iter().any(is_rstest) {
        quote!()
    } else {
        quote!(#[::rstest::rstest])
    };
    let tokio_attr = if attrs.iter().any(is_tokio_test) {
        quote!()
    } else {
        quote!(#[::tokio::test])
    };

    let ctx_binding = if ctx_is_mut {
        quote!(let mut __rstest_ctx)
    } else {
        quote!(let __rstest_ctx)
    };
    let ctx_pass = if ctx_is_mut {
        quote!(&mut __rstest_ctx)
    } else {
        quote!(&__rstest_ctx)
    };

    Ok(quote! {
        #rstest_attr
        #(#attrs)*
        #tokio_attr
        #vis #outer_sig {
            #inner_sig #block

            #ctx_binding = <#ctx_ty as ::test_context::AsyncTestContext>::setup().await;
            let __rstest_ctx_result = ::futures::future::FutureExt::catch_unwind(
                ::std::panic::AssertUnwindSafe(__rstest_ctx_body(#ctx_pass, #(#forward),*))
            ).await;
            <#ctx_ty as ::test_context::AsyncTestContext>::teardown(__rstest_ctx).await;
            match __rstest_ctx_result {
                ::std::result::Result::Ok(__v) => __v,
                ::std::result::Result::Err(__e) => ::std::panic::resume_unwind(__e),
            }
        }
    })
}

fn is_rstest(attr: &Attribute) -> bool {
    attr.path()
        .segments
        .last()
        .is_some_and(|s| s.ident == "rstest")
}

fn is_tokio_test(attr: &Attribute) -> bool {
    let segments = &attr.path().segments;
    let ends_with_test = segments.last().is_some_and(|s| s.ident == "test");
    let has_tokio = segments.iter().any(|s| s.ident == "tokio");
    ends_with_test && has_tokio
}
