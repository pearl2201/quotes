#[warn(unused_imports)]
extern crate reqwest;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
extern crate rand;

use rand::Rng;
use regex::Regex;
use scraper::{Html, Selector};
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Quote {
    author: String,
    tags: Vec<String>,
    content: String,
    likes: i64,
}

fn string_to_static_str(s: String) -> &'static str {
    Box::leak(s.into_boxed_str())
}

const RESULT_FILE_PATH: &str = "docs/data/quotes.json";
const URL_CRAWL: &str = "https://www.goodreads.com/quotes";

fn main() {
    if !Path::new(RESULT_FILE_PATH).exists() {
        crawl_quotes(URL_CRAWL);
    }
    let quote = random_quotes();
    print_quote(&quote);
}

fn print_quote(quote: &Quote) {
    println!("Quote: {}", quote.content);
    println!("Author: {}", quote.author);
    println!("Tags: {:?}", quote.tags);
    println!("Likes: {:?}", quote.likes);
}

fn random_quotes() -> Quote {
    let mut file = File::open(RESULT_FILE_PATH).unwrap();
    let mut content = String::new();
    file.read_to_string(&mut content).unwrap();
    let quotes: Vec<Quote> = serde_json::from_str(&content).unwrap();
    let mut rng = rand::thread_rng();
    let index = rng.gen_range(0, quotes.len());
    let quote = &quotes[index];
    return quote.clone();
}

fn get_count_page(uri_quote: &str) -> i64 {
    let client = reqwest::Client::new();

    let body_html = client
        .get(string_to_static_str(uri_quote.to_string()))
        .send()
        .unwrap()
        .text()
        .unwrap();

    let document = Html::parse_document(&body_html);
    let selector = Selector::parse(".leftContainer ").unwrap();
    let container_items: Vec<scraper::element_ref::ElementRef<'_>> =
        document.select(&selector).collect();
    let link_selector = Selector::parse("a").unwrap();
    let last_div: Vec<scraper::element_ref::ElementRef<'_>> = container_items
        [container_items.len() - 1]
        .select(&link_selector)
        .collect();

    let page_count: i64 = (last_div[last_div.len() - 2].inner_html()).parse().unwrap();
    return page_count;
}

fn crawl_quotes(uri_quote: &str) {
    let mut quotes: Vec<Quote> = Vec::new();
    let _re = Regex::new(r"“(.+?)”").unwrap();
    let re_likes = Regex::new(r"(\d+)").unwrap();
    let re_html = Regex::new(r"<[^\\n]{0,9}>").unwrap();
    let page_count = get_count_page(uri_quote);
    for i in 1..page_count {
        println!("_______________________________________________________________");
        println!("[*] Parse page: {}", i);
        let client = reqwest::Client::new();

        let mut owned_string: String = uri_quote.to_owned();
        owned_string.push_str(string_to_static_str(format!("?page={}", i)));
        let body_html = client
            .get(string_to_static_str(owned_string))
            .send()
            .unwrap()
            .text()
            .unwrap();

        let document = Html::parse_document(&body_html);
        let selector = Selector::parse(".quoteDetails").unwrap();
        let mut index_parse = 0;
        for element in document.select(&selector) {
            index_parse = index_parse + 1;

            let quote_text_selector = element
                .select(&Selector::parse(".quoteText").unwrap())
                .next()
                .unwrap();
            let quote_text = quote_text_selector.inner_html();

            let index_left: usize = (quote_text.find('“').unwrap()) as usize + 3usize;
            let index_right: usize = quote_text.find('”').unwrap() as usize;

            let quote_content;
            unsafe {
                quote_content = quote_text.get_unchecked(index_left..index_right);
            }
            let quote_content = re_html.replace_all(quote_content, "");

            let author = quote_text_selector
                .select(&Selector::parse(".authorOrTitle").unwrap())
                .next()
                .unwrap()
                .inner_html();
            let quote_author = author.trim();

            let tag_selectors_cmd = &Selector::parse(".quoteFooter .left a").unwrap();
            let mut tags: Vec<String> = Vec::new();
            for tag_selector in element.select(&tag_selectors_cmd) {
                tags.push(tag_selector.inner_html());
            }

            let like_selector_cmd = &Selector::parse(".quoteFooter .right a").unwrap();
            let likes_text = element
                .select(&like_selector_cmd)
                .next()
                .unwrap()
                .inner_html();
            let likes_capture = re_likes.captures(&likes_text).unwrap();
            let quote_likes_count: i64 = (&likes_capture[1]).parse().unwrap();

            quotes.push(Quote {
                author: quote_author.to_string(),
                content: quote_content.to_string(),
                tags: tags,
                likes: quote_likes_count,
            });
        }
    }
    let serialized = serde_json::to_string(&quotes).unwrap();
    {
        let mut file = File::create(RESULT_FILE_PATH).unwrap();
        file.write_all(serialized.as_bytes()).unwrap();
        drop(file);
    }
}
