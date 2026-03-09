(() => {
  const installTitles = {
    macos: "Terminal - macOS",
    linux: "Terminal - Linux",
    windows: "PowerShell - Windows"
  };

  const terminalLines = [
    { text: "$ omnimon --overview", color: "text" },
    { text: "", color: "text" },
    { text: "  SYSTEM STATUS                OmniMon v6.1.0", color: "cyan" },
    { text: "  -----------------------------------------", color: "text-dim" },
    { text: "  CPU     58.3% total  |  6 cores active", color: "green" },
    { text: "  Memory  11.4 / 16 GB | grouped process view", color: "amber" },
    { text: "  Alerts  grouped + debounced health spikes", color: "text" },
    { text: "", color: "text" },
    { text: "  HIGHLIGHTS", color: "cyan" },
    { text: "  -----------------------------------------", color: "text-dim" },
    { text: "  signed releases   Ed25519 + SHA-256", color: "text" },
    { text: "  AI chat           token streaming + 7 presets", color: "text" },
    { text: "  tool calling      process, network, security", color: "text" },
    { text: "  UI telemetry      native icons + clickable dashboards", color: "text" },
    { text: "", color: "text" },
    { text: "  OK release integrity verified | shield active", color: "green" }
  ];

  const colorMap = {
    cyan: "color: #00f0ff",
    green: "color: #00ff88",
    amber: "color: #ffb800",
    red: "color: #ff3366",
    text: "color: #e2e8f0",
    "text-dim": "color: #475569"
  };

  const prefersReducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)").matches;

  function renderTerminal() {
    const output = document.getElementById("terminal-output");
    if (!output) return;
    output.innerHTML = terminalLines
      .map((line) => {
        const safe = (line.text || "\u00A0")
          .replace(/&/g, "&amp;")
          .replace(/</g, "&lt;")
          .replace(/>/g, "&gt;");
        return `<div style="${colorMap[line.color] || colorMap.text};white-space:pre">${safe}</div>`;
      })
      .join("") + '<div class="typing-cursor" style="color:#e2e8f0">$ </div>';
  }

  function animateCounters() {
    const counters = document.querySelectorAll("[data-count]");
    if (!counters.length) return;
    const observer = new IntersectionObserver((entries) => {
      entries.forEach((entry) => {
        if (!entry.isIntersecting) return;
        const el = entry.target;
        const target = parseInt(el.dataset.count || "0", 10);
        const suffix = el.dataset.suffix || "";
        if (prefersReducedMotion) {
          el.textContent = target.toLocaleString() + suffix;
          observer.unobserve(el);
          return;
        }
        const duration = 900;
        const start = performance.now();
        const update = (now) => {
          const progress = Math.min((now - start) / duration, 1);
          const eased = 1 - Math.pow(1 - progress, 3);
          el.textContent = Math.floor(target * eased).toLocaleString() + suffix;
          if (progress < 1) requestAnimationFrame(update);
        };
        requestAnimationFrame(update);
        observer.unobserve(el);
      });
    }, { threshold: 0.45 });
    counters.forEach((el) => observer.observe(el));
  }

  function setupInstallTabs() {
    const tabs = document.querySelectorAll(".install-tab");
    const installPanels = document.querySelectorAll("[data-install]");
    const installTitleEl = document.getElementById("install-title");
    tabs.forEach((tab) => {
      tab.addEventListener("click", () => {
        const os = tab.dataset.os;
        tabs.forEach((item) => {
          item.classList.remove("active", "border-cyan", "text-cyan");
          item.classList.add("text-text-muted");
        });
        tab.classList.add("active", "border-cyan", "text-cyan");
        tab.classList.remove("text-text-muted");
        installPanels.forEach((panel) => panel.classList.toggle("hidden", panel.dataset.install !== os));
        if (installTitleEl) installTitleEl.textContent = installTitles[os] || "";
      });
    });
  }

  function detectOS() {
    const btn = document.getElementById("download-btn");
    const osLabel = document.getElementById("download-os");
    if (!btn || !osLabel) return;
    const ua = navigator.userAgent.toLowerCase();
    const platform = (navigator.platform || "").toLowerCase();
    let os = "";
    let label = "";
    if (platform.includes("mac") || ua.includes("macintosh")) {
      os = "macos";
      label = "macOS";
    } else if (platform.includes("win") || ua.includes("windows")) {
      os = "windows";
      label = "Windows";
    } else if (ua.includes("linux") || ua.includes("ubuntu") || ua.includes("debian")) {
      os = "linux";
      label = "Linux";
    }
    if (!os) return;
    osLabel.textContent = `(${label})`;
    const directUrl = btn.getAttribute(`data-${os}`);
    if (directUrl) btn.href = directUrl;
    document.querySelector(`.install-tab[data-os="${os}"]`)?.click();
  }

  function applyNavbarShadow() {
    const navbar = document.getElementById("navbar");
    if (!navbar) return;
    const toggle = () => navbar.classList.toggle("shadow-lg", window.scrollY > 50);
    toggle();
    window.addEventListener("scroll", toggle, { passive: true });
  }

  function activateTooltips() {
    document.querySelectorAll(".comparison-tooltip-trigger").forEach((trigger) => {
      const tooltip = trigger.querySelector(".comparison-tooltip");
      const icon = trigger.querySelector("svg");
      if (!tooltip || !icon) return;
      document.body.appendChild(tooltip);
      const show = () => {
        const rect = icon.getBoundingClientRect();
        const maxWidth = Math.min(280, window.innerWidth - 24);
        tooltip.style.maxWidth = `${maxWidth}px`;
        let left = rect.right + 10;
        if (left + maxWidth > window.innerWidth - 12) {
          left = Math.max(12, window.innerWidth - maxWidth - 12);
        }
        let top = rect.top + rect.height / 2;
        top = Math.max(40, Math.min(top, window.innerHeight - 40));
        tooltip.style.left = `${left}px`;
        tooltip.style.top = `${top}px`;
        tooltip.style.transform = "translateY(-50%)";
        tooltip.style.opacity = "1";
        tooltip.style.pointerEvents = "auto";
      };
      const hide = () => {
        tooltip.style.opacity = "0";
        tooltip.style.pointerEvents = "none";
      };
      trigger.addEventListener("mouseenter", show);
      trigger.addEventListener("focus", show);
      trigger.addEventListener("mouseleave", hide);
      trigger.addEventListener("blur", hide);
      trigger.addEventListener("click", (event) => {
        event.preventDefault();
        if (tooltip.style.opacity === "1") {
          hide();
        } else {
          show();
        }
      });
    });
  }

  function createParticles() {
    if (prefersReducedMotion) return;
    const container = document.getElementById("particles");
    if (!container) return;
    const count = window.innerWidth < 768 ? 8 : 14;
    for (let i = 0; i < count; i += 1) {
      const particle = document.createElement("div");
      particle.className = "particle";
      particle.style.left = `${Math.random() * 100}%`;
      particle.style.top = `${Math.random() * 100}%`;
      particle.style.opacity = (Math.random() * 0.28 + 0.08).toString();
      particle.style.animation = `drift ${6 + Math.random() * 8}s ease-in-out ${Math.random() * 3}s infinite`;
      if (Math.random() > 0.82) {
        particle.style.width = "3px";
        particle.style.height = "3px";
        particle.style.background = "#00ff88";
      }
      container.appendChild(particle);
    }
  }

  renderTerminal();
  animateCounters();
  setupInstallTabs();
  detectOS();
  applyNavbarShadow();
  activateTooltips();
  createParticles();
})();
