//! SuicaoDex Aidoku source
//! API: https://redive.suicaodex.com  (SuicaoDex production REST API)
#![no_std]

use aidoku::{
    alloc::{String, Vec, format},
    imports::net::Request,
    prelude::*,
    Chapter, Listing, ListingProvider, Manga, MangaPageResult, MangaStatus,
    Page, PageContent, Source, Result,
};
use serde_json::Value;

const API_BASE: &str = "https://redive.suicaodex.com";
const PAGE_SIZE: u32 = 24;

// ─── Listing IDs ────────────────────────────────────────────────────────────
const LISTING_LATEST: &str = "latest";
const LISTING_POPULAR: &str = "popular";

// ─── URL encode ─────────────────────────────────────────────────────────────
fn encode_component(s: &str) -> String {
    let mut out = String::new();
    for byte in s.as_bytes() {
        let c = *byte;
        if c.is_ascii_alphanumeric() || c == b'-' || c == b'_' || c == b'.' || c == b'~' {
            out.push(c as char);
        } else {
            out.push('%');
            out.push(hex((c >> 4) & 0x0f));
            out.push(hex(c & 0x0f));
        }
    }
    out
}

#[inline]
fn hex(n: u8) -> char {
    if n < 10 { (b'0' + n) as char } else { (b'A' + n - 10) as char }
}

// ─── HTTP helpers ────────────────────────────────────────────────────────────
fn get_json(path: &str) -> Result<Value> {
    let url = format!("{}{}", API_BASE, path);
    let resp = Request::get(&url)?
        .header("Accept", "application/json")
        .header("User-Agent", "Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) AppleWebKit/605.1.15")
        .header("Origin", "https://suicaodex.com")
        .header("Referer", "https://suicaodex.com/")
        .send()?;
    let status = resp.status_code();
    let body = resp.get_string()?;
    if !(200..300).contains(&status) {
        let snippet: String = body.chars().take(160).collect();
        bail!("HTTP {} từ {}: {}", status, url, snippet);
    }
    match serde_json::from_str::<Value>(&body) {
        Ok(v) => Ok(v),
        Err(_) => {
            let snippet: String = body.chars().take(160).collect();
            bail!("Không phải JSON ({}): {}", url, snippet)
        }
    }
}

// ─── Tiny helpers ────────────────────────────────────────────────────────────
fn str_of<'a>(v: &'a Value, key: &str) -> &'a str {
    v.get(key).and_then(Value::as_str).unwrap_or("")
}

fn opt_str(v: &Value, key: &str) -> Option<String> {
    v.get(key)
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(String::from)
}

fn array_names(v: &Value, key: &str) -> Option<Vec<String>> {
    let arr = v.get(key)?.as_array()?;
    let names: Vec<String> = arr
        .iter()
        .filter_map(|item| item.get("name").and_then(Value::as_str).map(String::from))
        .collect();
    if names.is_empty() { None } else { Some(names) }
}

fn total_pages(v: &Value) -> i64 {
    v.get("meta")
        .and_then(|m| m.get("totalPages"))
        .and_then(Value::as_i64)
        .unwrap_or(1)
}

/// Parse "2026-08-29T00:00:00.000Z" → Unix seconds (very small, fast)
fn parse_iso_timestamp(s: &str) -> Option<i64> {
    // expect at least "YYYY-MM-DDTHH:MM:SS"
    let b = s.as_bytes();
    if b.len() < 19 { return None; }

    let year  = parse_u32(&b[0..4])? as i64;
    let month = parse_u32(&b[5..7])? as i64;
    let day   = parse_u32(&b[8..10])? as i64;
    let hour  = parse_u32(&b[11..13])? as i64;
    let min   = parse_u32(&b[14..16])? as i64;
    let sec   = parse_u32(&b[17..19])? as i64;

    // Days from epoch to year-01-01
    let y = year - 1;
    let days = y * 365 + y / 4 - y / 100 + y / 400
        + days_before_month(year, month) + day - 1
        - 719162; // offset from 0001-01-01 to 1970-01-01

    Some(days * 86400 + hour * 3600 + min * 60 + sec)
}

fn days_before_month(year: i64, month: i64) -> i64 {
    const DAYS: [i64; 13] = [0, 0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334];
    let leap_add = if month > 2 && is_leap(year) { 1 } else { 0 };
    DAYS[month as usize] + leap_add
}

fn is_leap(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

fn parse_u32(b: &[u8]) -> Option<u32> {
    let mut n: u32 = 0;
    for &c in b {
        if c < b'0' || c > b'9' { return None; }
        n = n * 10 + (c - b'0') as u32;
    }
    Some(n)
}

// ─── Manga mapping ──────────────────────────────────────────────────────────
fn manga_from(v: &Value) -> Manga {
    let cover = v.get("cover")
        .and_then(|c| c.get("url"))
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(String::from);

    let id   = str_of(v, "id");
    let slug = str_of(v, "slug");

    Manga {
        key:     String::from(id),
        title:   String::from(str_of(v, "title")),
        cover,
        authors: array_names(v, "authors"),
        tags:    array_names(v, "tags"),
        url: if !id.is_empty() {
            Some(format!("https://suicaodex.com/manga/{}/{}", id, slug))
        } else {
            None
        },
        ..Default::default()
    }
}

fn parse_manga_page(v: &Value) -> Vec<Manga> {
    v.get("data")
        .and_then(Value::as_array)
        .map(|arr| arr.iter().map(manga_from).collect())
        .unwrap_or_default()
}

// ─── Chapter mapping ────────────────────────────────────────────────────────
fn parse_chapters(v: &Value) -> Vec<Chapter> {
    let Some(items) = v.get("data").and_then(Value::as_array) else {
        return Vec::new();
    };
    items.iter().map(|item| {
        let groups: Vec<String> = item.get("groups")
            .and_then(Value::as_array)
            .map(|arr| arr.iter()
                .filter_map(|g| g.get("name").and_then(Value::as_str).map(String::from))
                .collect())
            .unwrap_or_default();

        let scanlators = if groups.is_empty() { None } else { Some(groups) };

        let chapter_number = item.get("number")
            .and_then(Value::as_str)
            .and_then(|s| s.parse::<f32>().ok());

        let volume_number = item.get("volume")
            .and_then(Value::as_str)
            .and_then(|s| s.parse::<f32>().ok());

        let date_uploaded = item.get("publishedAt")
            .and_then(Value::as_str)
            .and_then(parse_iso_timestamp);

        Chapter {
            key:            String::from(str_of(item, "id")),
            title:          opt_str(item, "title"),
            chapter_number,
            volume_number,
            date_uploaded,
            scanlators,
            language:       opt_str(item, "language"),
            ..Default::default()
        }
    }).collect()
}

// ─── Source struct ──────────────────────────────────────────────────────────
struct SuicaoDex;

impl Source for SuicaoDex {
    fn new() -> Self { Self }

    fn get_search_manga_list(
        &self,
        query: Option<String>,
        page: i32,
        _filters: Vec<aidoku::FilterValue>,
    ) -> Result<MangaPageResult> {
        let page = page.max(1);
        let mut path = format!(
            "/v1/manga?page={}&limit={}&sort=latestChapter&direction=desc&hasChapter=true",
            page, PAGE_SIZE
        );
        if let Some(q) = query.as_deref().filter(|s| !s.trim().is_empty()) {
            path.push_str("&q=");
            path.push_str(&encode_component(q.trim()));
        }
        let data = get_json(&path)?;
        Ok(MangaPageResult {
            entries: parse_manga_page(&data),
            has_next_page: (page as i64) < total_pages(&data),
        })
    }

    fn get_manga_update(
        &self,
        mut manga: Manga,
        needs_details: bool,
        needs_chapters: bool,
    ) -> Result<Manga> {
        if needs_details {
            let d = get_json(&format!("/v1/manga/{}", manga.key))?;
            manga.title       = String::from(str_of(&d, "title"));
            manga.description = opt_str(&d, "description");
            manga.authors     = array_names(&d, "authors");
            manga.tags        = array_names(&d, "tags");
            manga.status = match str_of(&d, "status") {
                "ongoing"   => MangaStatus::Ongoing,
                "completed" => MangaStatus::Completed,
                "hiatus"    => MangaStatus::Hiatus,
                "cancelled" => MangaStatus::Cancelled,
                _           => MangaStatus::Unknown,
            };
            if let Some(url) = d.get("cover")
                .and_then(|c| c.get("url"))
                .and_then(Value::as_str)
                .filter(|s| !s.is_empty())
            {
                manga.cover = Some(String::from(url));
            }
            let id   = str_of(&d, "id");
            let slug = str_of(&d, "slug");
            if !id.is_empty() {
                manga.url = Some(format!("https://suicaodex.com/manga/{}/{}", id, slug));
            }
        }

        if needs_chapters {
            let first = get_json(&format!(
                "/v1/manga/{}/chapters?page=1&limit=100&sort=reading&direction=desc",
                manga.key
            ))?;
            let total = total_pages(&first).clamp(1, 50);
            let mut all = parse_chapters(&first);

            for p in 2..=total {
                let next = get_json(&format!(
                    "/v1/manga/{}/chapters?page={}&limit=100&sort=reading&direction=desc",
                    manga.key, p
                ))?;
                all.extend(parse_chapters(&next));
            }
            manga.chapters = Some(all);
        }

        Ok(manga)
    }

    fn get_page_list(&self, _manga: Manga, chapter: Chapter) -> Result<Vec<Page>> {
        let d = get_json(&format!("/v1/chapters/{}", chapter.key))?;
        let pages_val = d.get("pages").and_then(Value::as_array);
        let Some(items) = pages_val else {
            bail!("Chapter '{}': 'pages' field missing in API response", chapter.key);
        };

        let mut pages: Vec<Page> = Vec::new();
        // sort by position to guarantee order
        let mut sorted: Vec<(i64, &Value)> = items.iter().map(|item| {
            let pos = item.get("position").and_then(Value::as_i64).unwrap_or(0);
            (pos, item)
        }).collect();
        sorted.sort_by_key(|(pos, _)| *pos);

        for (_, item) in &sorted {
            let url = item.get("optimizedUrl")
                .and_then(Value::as_str).filter(|s| !s.is_empty())
                .or_else(|| item.get("replicaUrl").and_then(Value::as_str).filter(|s| !s.is_empty()))
                .or_else(|| item.get("url").and_then(Value::as_str).filter(|s| !s.is_empty()))
                .unwrap_or("");
            if !url.is_empty() {
                pages.push(Page {
                    content: PageContent::url(String::from(url)),
                    ..Default::default()
                });
            }
        }

        if pages.is_empty() {
            bail!("Chapter '{}': no page images found", chapter.key);
        }
        Ok(pages)
    }
}

// ─── Listings (latest & popular) ────────────────────────────────────────────
impl ListingProvider for SuicaoDex {
    fn get_manga_list(&self, listing: Listing, page: i32) -> Result<MangaPageResult> {
        let page = page.max(1);
        let path = match listing.id.as_str() {
            LISTING_POPULAR => format!(
                "/v1/manga?page={}&limit={}&sort=latestChapter&direction=desc&hasChapter=true",
                page, PAGE_SIZE
            ),
            _ /* latest */ => {
                // Use /v1/chapters/latest to get recently updated manga
                let data = get_json(&format!(
                    "/v1/chapters/latest?page={}&limit={}",
                    page, PAGE_SIZE
                ))?;

                // Build a deduplicated manga list from latest chapters
                let mut seen_ids: Vec<String> = Vec::new();
                let mut entries: Vec<Manga> = Vec::new();

                if let Some(chapters) = data.get("data").and_then(Value::as_array) {
                    for ch in chapters {
                        if let Some(m) = ch.get("manga") {
                            let id = str_of(m, "id");
                            if id.is_empty() { continue; }
                            if seen_ids.iter().any(|s| s == id) { continue; }
                            seen_ids.push(String::from(id));
                            entries.push(manga_from(m));
                        }
                    }
                }

                let has_next = (page as i64) < total_pages(&data);
                return Ok(MangaPageResult { entries, has_next_page: has_next });
            }
        };
        let data = get_json(&path)?;
        Ok(MangaPageResult {
            entries: parse_manga_page(&data),
            has_next_page: (page as i64) < total_pages(&data),
        })
    }
}

register_source!(SuicaoDex, ListingProvider);
