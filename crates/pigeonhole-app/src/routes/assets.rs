//! The one script, served from its own route: Autumn's CSP is
//! `script-src 'self'`, so neither an inline `<script>` nor `hx-on` runs.

use autumn_web::prelude::*;

/// The script: the board canvas, its hover and click, and clearing a form after htmx posts it.
pub const SCRIPT: &str = r"
(function () {
  var C = { empty: '#1a2029', one: '#3fd0b6', two: '#8f7cf0', hot: '#f0b35a', open: '#ffffff' };
  var pic = null, canvas = null, ctx = null, hover = null, cell = 5;
  function count(ch) { var c = ch.charCodeAt(0); return c <= 57 ? c - 48 : c - 87; }
  function draw() {
    if (!pic || !ctx) return;
    ctx.fillStyle = '#0f1318'; ctx.fillRect(0, 0, 640, 640);
    var open = {}; pic.open.forEach(function (s) { open[s] = true; });
    for (var s = 0; s < 16384; s++) {
      var n = count(pic.occupancy[s]);
      ctx.fillStyle = open[s] ? C.open : n === 0 ? C.empty : n === 1 ? C.one : n === 2 ? C.two : C.hot;
      ctx.fillRect((s % 128) * cell, Math.floor(s / 128) * cell, cell - 1, cell - 1);
    }
  }
  function load() {
    fetch('/_board.json', { credentials: 'same-origin' })
      .then(function (r) { return r.json(); })
      .then(function (p) { pic = p; draw(); })
      .catch(function () {});
  }
  function at(e) {
    var r = canvas.getBoundingClientRect();
    var x = Math.floor((e.clientX - r.left) / r.width * 128), y = Math.floor((e.clientY - r.top) / r.height * 128);
    if (x < 0 || y < 0 || x > 127 || y > 127) return -1;
    return y * 128 + x;
  }
  function pad(s) { s = '' + s; while (s.length < 5) s = '0' + s; return s; }
  function init() {
    canvas = document.getElementById('board');
    if (!canvas) return;
    ctx = canvas.getContext('2d');
    hover = document.getElementById('hover');
    canvas.addEventListener('mousemove', function (e) {
      var s = at(e); if (s < 0 || !pic) return;
      var n = count(pic.occupancy[s]);
      hover.textContent = 'slot ' + pad(s) + ' · row ' + Math.floor(s / 128) + ', col ' + (s % 128) +
        ' · ' + (n === 0 ? 'empty' : n + (n === 1 ? ' pigeon' : ' pigeons')) +
        (pic.open.indexOf(s) >= 0 ? ' · open' : '');
    });
    canvas.addEventListener('click', function (e) { var s = at(e); if (s >= 0) location.href = '/slot/' + s; });
    load();
    try { var es = new EventSource('/stream'); es.onmessage = load; } catch (e) {}
  }
  document.addEventListener('htmx:afterRequest', function (e) {
    var f = e.target;
    if (f && f.matches && f.matches('form[data-reset]') && e.detail.successful) f.reset();
  });
  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', init); else init();
  window.pigeonhole = { load: load };
})();
";

/// `/pigeonhole.js`.
#[get("/pigeonhole.js")]
#[public]
pub async fn script() -> impl IntoResponse {
    (
        [
            ("content-type", "text/javascript; charset=utf-8"),
            ("cache-control", "public, max-age=300"),
        ],
        SCRIPT,
    )
}

#[cfg(test)]
mod tests {
    use super::SCRIPT;

    #[test]
    fn the_script_has_no_eval_and_draws_the_board() {
        assert!(!SCRIPT.contains("eval("));
        assert!(!SCRIPT.contains("new Function"));
        assert!(SCRIPT.contains("window.pigeonhole"));
        assert!(SCRIPT.contains("/_board.json"));
    }
}
