//! The shell every page renders into, and Pigeonhole's look.
//!
//! A sorting rack in a dim room: slate, and the only colour is the
//! iridescence of a pigeon's neck — teal shading to violet — on the slots that
//! hold somebody. Crowded slots run warm. Nothing else competes with the board.
//!
//! Inline styles only; the one script is served from its own route because
//! Autumn's CSP is `script-src 'self'`.

use autumn_web::{HTMX_CSRF_JS_PATH, HTMX_JS_PATH, HTMX_SSE_JS_PATH, Markup, PreEscaped, html};

/// The stylesheet.
pub const STYLE: &str = r#"
:root{
  --ground:#0d1015; --ground-2:#12161d; --panel:#151a22; --rule:#232a35; --rule-2:#303949;
  --ink:#e6ebf2; --dim:#9aa6b6; --faint:#6b7687;
  --neck:#3fd0b6; --neck-2:#8f7cf0; --hot:#f0b35a; --empty:#1a2029;
  --good:#7fc98f; --bad:#ef6b5a;
  --serif:"Iowan Old Style","Palatino Linotype",Palatino,Georgia,ui-serif,serif;
  --sans:ui-sans-serif,system-ui,-apple-system,"Segoe UI",Roboto,sans-serif;
  --mono:ui-monospace,SFMono-Regular,Menlo,"Cascadia Mono",monospace;
}
*{box-sizing:border-box} html,body{margin:0;padding:0}
body{background:radial-gradient(1100px 700px at 15% -10%,rgba(63,208,182,.06),transparent 60%),radial-gradient(900px 600px at 100% 100%,rgba(143,124,240,.06),transparent 60%),var(--ground);
  background-attachment:fixed;color:var(--ink);font-family:var(--sans);font-size:15px;line-height:1.55;min-height:100vh}
a{color:var(--neck);text-decoration:none} a:hover{text-decoration:underline}
code,.mono{font-family:var(--mono);font-size:.92em}
.wrap{max-width:1360px;margin:0 auto;padding:0 22px 60px}
header.top{display:flex;align-items:baseline;gap:18px;flex-wrap:wrap;padding:22px 0 14px;border-bottom:1px solid var(--rule);margin-bottom:22px}
header.top h1{font-family:var(--serif);font-weight:400;letter-spacing:.3em;text-transform:uppercase;font-size:24px;margin:0;color:var(--ink)}
header.top h1 a{color:inherit} header.top h1 a:hover{text-decoration:none}
header.top .tag{color:var(--dim);font-family:var(--serif);font-style:italic;font-size:14px}
header.top nav{margin-left:auto;display:flex;gap:20px;font-size:13.5px}
header.top nav a{color:var(--dim)} header.top nav a:hover{color:var(--neck)}
h2{font-family:var(--serif);font-weight:400;font-size:27px;margin:0 0 6px}
h3{font-size:11px;letter-spacing:.2em;text-transform:uppercase;color:var(--faint);margin:0 0 10px;font-weight:600}
p.lede{color:var(--dim);max-width:74ch;margin:0 0 18px}
.hint{color:var(--faint);font-size:12.5px}
.grid{display:grid;grid-template-columns:minmax(0,1fr) 340px;gap:22px;align-items:start}
@media (max-width:1100px){.grid{grid-template-columns:1fr}}
.panel{background:var(--panel);border:1px solid var(--rule);border-radius:6px;padding:16px 18px 18px}
.panel + .panel{margin-top:16px}
.stats{display:grid;grid-template-columns:repeat(2,1fr);gap:10px}
.stat{background:var(--ground-2);border:1px solid var(--rule);border-radius:5px;padding:9px 11px}
.stat b{display:block;font-family:var(--serif);font-weight:400;font-size:23px;line-height:1.15;font-variant-numeric:tabular-nums}
.stat span{font-size:11px;color:var(--faint);letter-spacing:.06em}
.stat.lit b{color:var(--neck)}
.btn{display:inline-block;font:inherit;font-size:14px;border:1px solid #2f6f66;background:linear-gradient(180deg,#173a36,#10282a);color:#bff3e9;padding:8px 15px;border-radius:5px;cursor:pointer}
.btn:hover{border-color:var(--neck);box-shadow:0 0 14px rgba(63,208,182,.22)}
.btn[disabled]{opacity:.45;cursor:not-allowed;box-shadow:none}
.btn.quiet{border-color:var(--rule-2);color:var(--dim);background:var(--ground-2)}
input[type=text]{font:inherit;background:var(--ground-2);color:var(--ink);border:1px solid var(--rule-2);border-radius:5px;padding:8px 10px;width:100%}
input[type=text]:focus{outline:none;border-color:#2f6f66;box-shadow:0 0 0 3px rgba(63,208,182,.12)}
form.row{display:flex;gap:8px} form.row input{flex:1}
.err{color:var(--bad);font-size:13px;margin-top:6px} .ok{color:var(--good);font-size:13px;margin-top:6px}
.board-wrap{overflow-x:auto}
canvas#board{display:block;width:640px;height:640px;max-width:100%;aspect-ratio:1;image-rendering:pixelated;border-radius:4px;background:var(--empty);cursor:crosshair}
.legend{display:flex;gap:16px;align-items:center;font-size:12px;color:var(--faint);margin-top:10px;flex-wrap:wrap}
.legend i{display:inline-block;width:11px;height:11px;border-radius:2px;vertical-align:-1px;margin-right:5px}
.hover{font-family:var(--mono);font-size:12.5px;color:var(--dim);min-height:1.4em;margin-top:8px}
.empty{color:var(--faint);font-style:italic;padding:20px 6px}
.notes{list-style:none;margin:0;padding:0}
.notes li{padding:9px 2px;border-bottom:1px solid var(--rule);display:flex;gap:12px}
.notes li .when{color:var(--faint);font-size:12px;white-space:nowrap;font-family:var(--mono)}
.kv{display:grid;grid-template-columns:120px 1fr;gap:4px 10px;font-size:13.5px}
.kv dt{color:var(--faint)} .kv dd{margin:0;word-break:break-all}
.split{display:flex;height:12px;border-radius:6px;overflow:hidden;background:var(--empty);margin:10px 0 4px}
.split i{display:block;height:100%}
.split .mine{background:linear-gradient(90deg,var(--neck),var(--neck-2))} .split .theirs{background:repeating-linear-gradient(45deg,#2a3240 0 6px,#222833 6px 12px)}
.chips{display:flex;flex-wrap:wrap;gap:6px} .chip{font-family:var(--mono);font-size:12px;padding:2px 8px;border:1px solid var(--rule-2);border-radius:10px;color:var(--dim)}
table.occ{border-collapse:collapse;font-size:13.5px;font-variant-numeric:tabular-nums;width:100%}
table.occ td,table.occ th{padding:6px 10px;border-bottom:1px solid var(--rule);text-align:right} table.occ th{color:var(--faint);font-weight:600;font-size:11px;letter-spacing:.12em;text-transform:uppercase}
table.occ td:first-child,table.occ th:first-child{text-align:left}
.bar{height:6px;background:var(--empty);border-radius:3px;overflow:hidden;margin:8px 0} .bar i{display:block;height:100%;background:var(--neck)}
.verdict{font-family:var(--serif);font-size:22px} .verdict.good{color:var(--good)} .verdict.bad{color:var(--bad)}
.prose{max-width:74ch} .prose p{color:var(--dim)} .prose h3{margin-top:26px}
footer{margin-top:40px;padding-top:14px;border-top:1px solid var(--rule);color:var(--faint);font-size:12.5px}
"#;

/// The page shell.
pub fn page(title: &str, csrf: Option<&str>, content: &Markup) -> Markup {
    html! {
        (PreEscaped("<!DOCTYPE html>"))
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                title { (title) " — Pigeonhole" }
                @if let Some(token) = csrf {
                    meta name="csrf-token" content=(token);
                }
                style { (PreEscaped(STYLE)) }
                script src=(HTMX_JS_PATH) defer {}
                script src=(HTMX_CSRF_JS_PATH) defer {}
                script src=(HTMX_SSE_JS_PATH) defer {}
                script src="/pigeonhole.js" defer {}
            }
            body {
                div class="wrap" {
                    header class="top" {
                        h1 { a href="/" { "Pigeonhole" } }
                        span class="tag" { "sixteen thousand holes, more pigeons than that" }
                        nav {
                            a href="/" { "The board" }
                            a href="/pigeon" { "Your pigeon" }
                            a href="/census" { "Census" }
                            a href="/about" { "How it works" }
                        }
                    }
                    (content)
                    footer {
                        "Every tenant hashes to one of 16,384 slots, and every slot is one SQLite file "
                        "that any number of tenants may share. Built on autumn-web 0.9's database fleet "
                        "in slot mode. "
                        a href="/about" { "How it works" } "."
                    }
                }
            }
        }
    }
}

/// Thousands-separated integer.
#[must_use]
pub fn commas(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// A byte count for people.
#[must_use]
pub fn bytes(n: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KiB", "MiB", "GiB"];
    let mut v = n;
    let mut unit = 0;
    while v >= 1024 && unit + 1 < UNITS.len() {
        v /= 1024;
        unit += 1;
    }
    format!("{} {}", commas(v), UNITS[unit])
}

/// A stat tile.
pub fn stat(value: impl maud::Render, label: &str, lit: bool) -> Markup {
    html! {
        div class=(if lit { "stat lit" } else { "stat" }) {
            b { (value) }
            span { (label) }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commas_group_thousands() {
        assert_eq!(commas(0), "0");
        assert_eq!(commas(999), "999");
        assert_eq!(commas(1_000), "1,000");
        assert_eq!(commas(1_234_567), "1,234,567");
    }

    #[test]
    fn bytes_scale() {
        assert_eq!(bytes(512), "512 B");
        assert_eq!(bytes(20_480), "20 KiB");
        assert_eq!(bytes(5 * 1024 * 1024 * 1024), "5 GiB");
    }

    #[test]
    fn the_page_loads_the_three_htmx_scripts_and_ours() {
        let html = page("x", Some("tok"), &html! { p { "hi" } }).into_string();
        for src in [
            HTMX_JS_PATH,
            HTMX_CSRF_JS_PATH,
            HTMX_SSE_JS_PATH,
            "/pigeonhole.js",
        ] {
            assert!(html.contains(src), "{src}");
        }
        assert!(html.contains(r#"name="csrf-token" content="tok""#));
    }
}
