(() => {
  async function initBlogSearch() {
    const input = document.querySelector("[data-blog-search-input]");
    const cards = document.querySelector("[data-blog-cards]");
    const status = document.querySelector("[data-blog-search-status]");
    const clearBtn = document.querySelector("[data-blog-search-clear]");
    if (!input || !cards || !status) return;

    const indexUrl = input.getAttribute("data-search-index");
    const locale = document.documentElement.lang === "es" ? "es" : "en";
    const strings = {
      en: { placeholder: "Search by version, title, feature, or text", empty: "No posts match your search.", all: "Showing all posts", result: "results", one: "result", clear: "Clear" },
      es: { placeholder: "Buscar por versión, título, función o texto", empty: "No hay posts que coincidan con tu búsqueda.", all: "Mostrando todos los posts", result: "resultados", one: "resultado", clear: "Limpiar" },
    }[locale];

    input.setAttribute("placeholder", strings.placeholder);
    if (clearBtn) clearBtn.textContent = strings.clear;

    const response = await fetch(indexUrl, { credentials: "same-origin" });
    const entries = await response.json();

    const escapeHtml = (value) => value
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;")
      .replace(/\"/g, "&quot;")
      .replace(/'/g, "&#39;");

    const render = (items, query) => {
      if (!items.length) {
        cards.innerHTML = `<div class="glass rounded-xl p-6 text-sm text-text-muted">${strings.empty}</div>`;
      } else {
        cards.innerHTML = items.map((item, index) => {
          const tags = (item.tags || []).slice(0, 4).map((tag) => `<span class="text-[10px] font-mono text-cyan bg-cyan/10 px-2 py-0.5 rounded-full border border-cyan/20">${escapeHtml(tag)}</span>`).join("");
          return `<a href="${escapeHtml(item.url)}" class="glass glass-hover rounded-xl p-6 block reveal reveal-delay-${Math.min(index + 1, 4)}" data-cursor-hover>
            <div class="flex items-start gap-4">
              <div class="min-w-0 flex-1">
                <div class="flex flex-wrap gap-1.5 mb-2">${tags}</div>
                <h3 class="text-lg font-bold mb-1">${escapeHtml(item.title)}</h3>
                <p class="text-sm text-text-muted line-clamp-2">${escapeHtml(item.summary)}</p>
                <div class="flex items-center gap-3 mt-3">
                  <time class="text-xs text-text-dim font-mono">${escapeHtml(item.date)}</time>
                  <span class="text-xs text-cyan font-semibold">${locale === 'es' ? 'Leer más ->' : 'Read more ->'}</span>
                </div>
              </div>
              <svg class="w-5 h-5 text-text-dim shrink-0 mt-2" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M9 18l6-6-6-6"></path></svg>
            </div>
          </a>`;
        }).join("");
      }
      const count = items.length;
      status.textContent = query ? `${count} ${count === 1 ? strings.one : strings.result}` : `${strings.all} (${count})`;
    };

    const run = () => {
      const query = input.value.trim().toLowerCase();
      if (!query) {
        render(entries, "");
        return;
      }
      const terms = query.split(/\s+/).filter(Boolean);
      const filtered = entries.filter((item) => {
        const haystack = [item.title, item.summary, item.content, item.version, ...(item.tags || [])].join(" ").toLowerCase();
        return terms.every((term) => haystack.includes(term));
      });
      render(filtered, query);
    };

    input.addEventListener("input", run);
    clearBtn?.addEventListener("click", () => { input.value = ""; run(); input.focus(); });
    render(entries, "");
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", initBlogSearch, { once: true });
  } else {
    initBlogSearch();
  }
})();
