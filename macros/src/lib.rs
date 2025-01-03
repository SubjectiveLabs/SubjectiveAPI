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
            let trigrams = name
                .bytes()
                .tuple_windows()
                .filter_map(|(a, b, c)| {
                    [a, b, c]
                        .into_iter()
                        .all(|byte| byte.is_ascii_alphabetic())
                        .then_some([
                            a.to_ascii_lowercase(),
                            b.to_ascii_lowercase(),
                            c.to_ascii_lowercase(),
                        ])
                })
                .collect_vec();
            (trigrams, icon)
        })
        .collect_vec();
    let len = data.len();
    let all_trigrams = data
        .iter()
        .flat_map(|(trigrams, _)| trigrams)
        .unique()
        .collect_vec();
    let trigrams_len = all_trigrams.len();
    let all_icons = data.iter().map(|(_, icon)| icon).unique().collect_vec();
    let icons_len = all_icons.len();
    let data = data
        .iter()
        .map(|(trigrams, icon)| {
            let counts = all_trigrams
                .iter()
                .map(|trigram| trigrams.contains(trigram))
                .collect_vec();
            quote! {
                (
                    [#(#counts),*],
                    #icon,
                )
            }
        })
        .collect_vec();
    let all_trigrams = all_trigrams
        .into_iter()
        .map(|trigram| {
            let [a, b, c] = trigram;
            quote! {
                [#a, #b, #c]
            }
        })
        .collect_vec();
    quote! {
        static ICONS: [&'static str; #icons_len] = [
            #(#all_icons),*
        ];
        static TRIGRAMS: [[u8; 3]; #trigrams_len] = [
            #(#all_trigrams),*
        ];
        static ICON_DATA: [([bool; #trigrams_len], &'static str); #len] = [
            #(#data),*
        ];
    }
    .into()
}
