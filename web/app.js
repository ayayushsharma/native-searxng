"use strict";
// native-searxng console: static page, one dynamic endpoint (/api/config).
// Token lives in localStorage, leaves only as an Authorization header.

const $ = (id) => document.getElementById(id);
let CFG = null;
let TOKEN = localStorage.getItem("sxng_token") || "";
let CATEGORY = "";

function esc(s) {
  return String(s ?? "").replace(/[&<>"']/g, (c) => ({
    "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;",
  }[c]));
}

function headers() {
  const h = {};
  if (TOKEN) h["Authorization"] = "Bearer " + TOKEN;
  return h;
}

function showErr(msg) {
  const e = $("err");
  if (!msg) { e.style.display = "none"; e.textContent = ""; return; }
  e.style.display = "block";
  e.textContent = msg;
}

// ---- boot: capabilities + auth mode from the server (never guessed) ----
async function boot() {
  const r = await fetch("/api/config");
  CFG = await r.json();
  $("ver").textContent = "v" + CFG.version;
  if (CFG.authRequired) {
    const b = $("authbadge");
    b.style.display = ""; b.className = "badge on"; b.textContent = "secured";
    $("tokenbar").style.display = "";
    if (TOKEN) $("token").value = TOKEN;
  } else {
    const b = $("authbadge");
    b.style.display = ""; b.className = "badge off"; b.textContent = "dev open";
  }
  // category pills
  const pills = $("pills");
  pills.innerHTML = "";
  const mk = (name, label) => {
    const btn = document.createElement("button");
    btn.type = "button"; btn.className = "pill" + (name === CATEGORY ? " active" : "");
    btn.textContent = label || name;
    btn.onclick = () => {
      CATEGORY = (CATEGORY === name) ? "" : name;
      [...pills.children].forEach((x) => x.classList.remove("active"));
      if (CATEGORY) btn.classList.add("active");
    };
    pills.appendChild(btn);
  };
  mk("", "all");
  CFG.categories.forEach((c) => mk(c));
  $("footcat").textContent = CFG.engines.length + " engines · " + CFG.categories.join(" · ");
  // engines tab
  const byCat = {};
  CFG.engines.forEach((e) => { (byCat[e.category] = byCat[e.category] || []).push(e.name); });
  $("engsummary").textContent = CFG.engines.length + " native engines across " + CFG.categories.length + " categories. Order = merge weight.";
  $("englist").innerHTML = Object.keys(byCat).map((c) =>
    `<div class="catgroup"><h3>${esc(c)} <span class="count">${byCat[c].length}</span></h3>` +
    `<table class="eng">${byCat[c].map((n) => `<tr><td>${esc(n)}</td><td>${esc(transport(n))}</td></tr>`).join("")}</table></div>`
  ).join("");
  $("keys").innerHTML = "top: " + CFG.topKeys.map((k) => `<code>${esc(k)}</code>`).join(" ") +
    "<br>result: " + CFG.resultKeys.map((k) => `<code>${esc(k)}</code>`).join(" ");
  buildApiFields();
}

function transport(name) {
  // short transport notes for the engines table (mirrors engine docs)
  const chain = { boardreader: "2-step session", gmx: "2-step hash", chatnoir: "3-step chain", pubmed: "2-step ids→summary", open_meteo: "2-step geo→forecast" };
  if (chain[name]) return chain[name];
  if (["flask", "pipdocs", "cheatsh", "pkggodev", "tagesschau"].includes(name)) return "HTML/JS scrape";
  return "JSON API";
}

// ---- token ----
$("save").onclick = () => {
  TOKEN = $("token").value.trim();
  if (TOKEN) localStorage.setItem("sxng_token", TOKEN);
  showErr("");
};
$("forget").onclick = () => {
  TOKEN = ""; $("token").value = "";
  localStorage.removeItem("sxng_token");
};

// ---- tabs ----
document.querySelectorAll("nav.tabs button").forEach((b) => {
  b.onclick = () => {
    document.querySelectorAll("nav.tabs button").forEach((x) => x.classList.remove("active"));
    document.querySelectorAll("section.tab").forEach((x) => x.classList.remove("active"));
    b.classList.add("active");
    $("tab-" + b.dataset.tab).classList.add("active");
  };
});

// ---- search ----
$("form").onsubmit = async (ev) => {
  ev.preventDefault();
  const q = $("q").value.trim();
  if (!q) return;
  showErr("");
  $("meta").textContent = "searching…";
  $("results").innerHTML = "";
  const params = new URLSearchParams({ q, format: "json" });
  if (CATEGORY) params.set("categories", CATEGORY);
  const engs = $("engines").value.trim();
  if (engs) params.set("engines", engs);
  let r;
  try {
    r = await fetch("/search?" + params.toString(), { headers: headers() });
  } catch (e) {
    showErr("request failed: " + e.message);
    $("meta").textContent = "";
    return;
  }
  if (r.status === 401) {
    showErr("401 unauthorized — save a bearer token above (or start the server with one).");
    $("meta").textContent = "";
    return;
  }
  const d = await r.json();
  renderResults(d);
};

function renderResults(d) {
  const rs = d.results || [];
  const un = (d.unresponsive_engines || []).map((u) => Array.isArray(u) ? u[0] : u);
  $("meta").textContent = `${rs.length} result${rs.length === 1 ? "" : "s"} · query “${d.query || ""}”`;
  let html = "";
  (d.answers || []).forEach((a) => {
    html += `<div class="card answer"><span class="eng">answer</span><span class="crumb">${esc(a)}</span></div>`;
  });
  rs.forEach((x) => {
    const det = x.publishedDate ? ` <span class="crumb">· ${esc(x.publishedDate)}</span>` : "";
    const img = x.thumbnail ? `<img class="thumb" loading="lazy" src="${esc(x.thumbnail)}" alt="">` : "";
    html += `<div class="card">${img}<span class="eng">${esc(x.engine || "?")}</span>` +
      `<span class="crumb">${esc(x.url || "")}</span>${det}` +
      `<h3><a href="${esc(x.url || "#")}" target="_blank" rel="noopener">${esc(x.title || x.url || "")}</a></h3>` +
      (x.content ? `<p>${esc(x.content)}</p>` : "") + `</div>`;
  });
  if (!rs.length) html += `<div class="card"><p>No results.</p></div>`;
  if (un.length) html += `<div class="warnline">unresponsive: ${esc(un.join(", "))}</div>`;
  $("results").innerHTML = html;
}

// ---- api console ----
const FIELDS = {
  search: [["q", "flask request context"], ["categories", "it"], ["engines", ""]],
  fetch: [["url", "https://example.com"]],
  mcplist: [],
  mcpsearch: [["query", "flask request context"], ["categories", "it"], ["engines", ""], ["max_results", "5"]],
  mcpfetch: [["url", "https://example.com"]],
};
function buildApiFields() {
  const sel = $("endpoint").value;
  const box = $("apifields");
  box.innerHTML = "";
  (FIELDS[sel] || []).forEach(([name, ph]) => {
    const label = document.createElement("label");
    label.textContent = name;
    const inp = document.createElement("input");
    inp.name = name; inp.placeholder = ph; inp.value = ph;
    box.appendChild(label); box.appendChild(inp);
  });
}
$("endpoint").onchange = buildApiFields;
$("run").onclick = async () => {
  const sel = $("endpoint").value;
  const vals = {};
  document.querySelectorAll("#apifields input").forEach((i) => { if (i.value) vals[i.name] = i.value; });
  $("out").textContent = "// running…";
  try {
    let r;
    if (sel === "search") {
      r = await fetch("/search?" + new URLSearchParams({ ...vals, format: "json" }).toString(), { headers: headers() });
    } else if (sel === "fetch") {
      r = await fetch("/fetch?" + new URLSearchParams(vals).toString(), { headers: headers() });
    } else if (sel === "mcplist") {
      r = await fetch("/mcp", { method: "POST", headers: { ...headers(), "Content-Type": "application/json" },
        body: JSON.stringify({ jsonrpc: "2.0", id: 1, method: "tools/list", params: {} }) });
    } else {
      const tool = sel === "mcpsearch" ? "search" : "fetch";
      const args = { ...vals };
      if (args.max_results) args.max_results = parseInt(args.max_results, 10) || 5;
      r = await fetch("/mcp", { method: "POST", headers: { ...headers(), "Content-Type": "application/json" },
        body: JSON.stringify({ jsonrpc: "2.0", id: 1, method: "tools/call", params: { name: tool, arguments: args } }) });
    }
    const text = await r.text();
    let pretty = text;
    try { pretty = JSON.stringify(JSON.parse(text), null, 2); } catch (e) { /* raw */ }
    $("out").textContent = `// HTTP ${r.status}\n` + pretty;
  } catch (e) {
    $("out").textContent = "// request failed: " + e.message;
  }
};

boot();
