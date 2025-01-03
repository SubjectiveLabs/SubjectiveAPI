use std::iter::once;

use itertools::Itertools;
use macros::load_icon_data;
use worker::{Request, Response, RouteContext};

load_icon_data!("src/icon_data");

pub async fn choose(request: Request, _context: RouteContext<()>) -> worker::Result<Response> {
    let url = request.url()?;
    let Some(input) = url
        .query_pairs()
        .find_map(|(key, value)| (key == "name").then_some(value))
    else {
        return Response::error("Missing `name` parameter.", 400);
    };
    Response::from_json(&classify(input.as_ref()))
}

#[allow(clippy::cast_precision_loss)]
fn classify(name: &str) -> Vec<&'static str> {
    let result = ICONS
        .iter()
        .map(|icon| {
            (
                name.bytes()
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
                    .map(|trigram| {
                        (ICON_DATA
                            .iter()
                            .filter(|(vector, other)| {
                                vector
                                    .iter()
                                    .zip_eq(TRIGRAMS)
                                    .any(|(contains, other)| other == trigram && *contains)
                                    && other == icon
                            })
                            .count() as f32
                            / ICON_DATA.iter().filter(|(_, other)| other == icon).count() as f32)
                            .log2()
                    })
                    .chain(once(
                        (ICON_DATA.iter().filter(|(_, other)| other == icon).count() as f32
                            / ICONS.len() as f32)
                            .log2(),
                    ))
                    .sum::<f32>(),
                icon,
            )
        })
        .filter(|(score, _)| score.is_normal())
        .sorted_unstable_by(|(a, _), (b, _)| {
            b.partial_cmp(a)
                .expect("all of the scores should be comparable")
        })
        .take(10)
        .collect_vec();
    result.iter().map(|(_, icon)| **icon).collect_vec()
}
