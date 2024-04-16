use heck::{ToKebabCase, ToSnakeCase, ToUpperCamelCase};
use proc_macro::TokenStream;
use quote::{format_ident, quote};

const SPIN_TEST_NAME_PREFIX: &str = "spin_test_";

#[proc_macro_attribute]
pub fn spin_test(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let func_item = syn::parse_macro_input!(item as syn::ItemFn);
    let func_name = &func_item.sig.ident;

    let world_name = format!("{SPIN_TEST_NAME_PREFIX}{func_name}").to_kebab_case();
    let world_text = generate_world(func_name);

    let export_name = format_ident!("{}", world_name.to_snake_case());
    let export_impl = format_ident!("{}", world_name.to_upper_camel_case());

    let tokens = quote!(
        #func_item

        mod #export_name {
            ::spin_test_sdk::wit_bindgen::generate!({
                inline: #world_text,
                runtime_path: "::spin_test_sdk::wit_bindgen::rt",
            });

            struct #export_impl;

            impl Guest for #export_impl {
                fn #export_name() {
                    super::#func_name()
                }
            }

            export!(#export_impl);
        }
    );
    tokens.into()
}

fn generate_world(ident: &syn::Ident) -> proc_macro2::TokenStream {
    let world_name = format!("{SPIN_TEST_NAME_PREFIX}{ident}").to_kebab_case();
    let world_text = format!(
        r#"
        package test:test;

        world {world_name} {{
            export {world_name}: func();
        }}
    "#
    );

    let litstr = syn::LitStr::new(&world_text, proc_macro2::Span::call_site());
    quote! { #litstr }
}
