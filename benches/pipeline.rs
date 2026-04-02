use criterion::{Criterion, black_box, criterion_group, criterion_main};
use std::fs;

use ark_story_plaintext::lexer::{extract_param, tokenize_line};
use ark_story_plaintext::parser::{self, parse, strip_xml_tags};
use ark_story_plaintext::renderer::render;

fn load_all_inputs() -> String {
    let mut all = String::new();
    let mut entries: Vec<_> = fs::read_dir("inputs")
        .expect("inputs/ directory required for benchmarks")
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "txt"))
        .collect();
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        all.push_str(&fs::read_to_string(entry.path()).unwrap());
        all.push('\n');
    }
    all
}

fn bench_lexer(c: &mut Criterion) {
    let input = load_all_inputs();
    let lines: Vec<&str> = input.lines().collect();

    c.bench_function("lexer/tokenize_all_lines", |b| {
        b.iter(|| {
            for line in &lines {
                black_box(tokenize_line(line));
            }
        });
    });

    // Bench extract_param on a representative heavy tag
    let heavy_tag = r#"Sticker(id="st1", multi = true, text="1099年，哥伦比亚，麦克斯哥伦比亚特区郊外，", x=320,y=340, alignment="left", size=24, delay=0.04, width=640,block = true)"#;
    c.bench_function("lexer/extract_param", |b| {
        b.iter(|| {
            black_box(extract_param(heavy_tag, "text"));
            black_box(extract_param(heavy_tag, "id"));
            black_box(extract_param(heavy_tag, "multi"));
        });
    });
}

fn bench_parser(c: &mut Criterion) {
    let input = load_all_inputs();

    c.bench_function("parser/parse_all", |b| {
        b.iter(|| {
            black_box(parse(&input));
        });
    });

    let xml_text = r#"<color=#000000><i>"如若此后百年千年，来人漫步于繁星身侧，人们便要赞颂她的名。"</i></color>"#;
    c.bench_function("parser/strip_xml_tags", |b| {
        b.iter(|| {
            black_box(strip_xml_tags(xml_text));
        });
    });
}

fn bench_renderer(c: &mut Criterion) {
    let input = load_all_inputs();
    let events = parse(&input);

    c.bench_function("renderer/render_all", |b| {
        b.iter(|| {
            black_box(render(&events));
        });
    });
}

fn bench_full_pipeline(c: &mut Criterion) {
    let input = load_all_inputs();

    c.bench_function("full_pipeline", |b| {
        b.iter(|| {
            let events = parse(&input);
            black_box(render(&events));
        });
    });
}

criterion_group!(benches, bench_lexer, bench_parser, bench_renderer, bench_full_pipeline);
criterion_main!(benches);
