use itertools::Itertools;
use proc_macro::TokenStream;
use quote::quote;
use std::fs::read_to_string;
use syn::{LitStr, parse_macro_input};

#[proc_macro]
pub fn load_icon_data(input: TokenStream) -> TokenStream {
    let data = read_to_string(parse_macro_input!(input as LitStr).value()).unwrap();
    let data = data
        .lines()
        .map(|line| {
            let (name_len, rest) = line.split_once(' ').unwrap();
            let (name, icon) = rest.split_at(name_len.parse().unwrap());
            let digrams = name
                .bytes()
                .tuple_windows()
                .filter_map(|(a, b)| {
                    [a, b]
                        .into_iter()
                        .all(|byte| byte != b' ')
                        .then_some([
                            a.to_ascii_lowercase(),
                            b.to_ascii_lowercase(),
                        ])
                })
                .collect_vec();
            (digrams, icon)
        })
        .collect_vec();
    let len = data.len();
    let all_digrams = data
        .iter()
        .flat_map(|(digrams, _)| digrams)
        .unique()
        .collect_vec();
    let digrams_len = all_digrams.len();
    let all_icons = data.iter().map(|(_, icon)| icon).unique().collect_vec();
    let icons_len = all_icons.len();
    let data = data
        .iter()
        .map(|(digrams, icon)| {
            let counts = all_digrams
                .iter()
                .map(|digram| digrams.contains(digram))
                .collect_vec();
            quote! {
                (
                    [#(#counts),*],
                    #icon,
                )
            }
        })
        .collect_vec();
    let all_digrams = all_digrams
        .into_iter()
        .map(|digram| {
            let [a, b] = digram;
            quote! {
                [#a, #b]
            }
        })
        .collect_vec();
    quote! {
        static ICONS: [&'static str; #icons_len] = [
            #(#all_icons),*
        ];
        static DIGRAMS: [[u8; 2]; #digrams_len] = [
            #(#all_digrams),*
        ];
        static ICON_DATA: [([bool; #digrams_len], &'static str); #len] = [
            #(#data),*
        ];
    }
    .into()
}
