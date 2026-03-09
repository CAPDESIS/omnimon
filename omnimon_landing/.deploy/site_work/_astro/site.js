(() => {
  const cursor = document.getElementById("cursor-scope");
  const finePointer = window.matchMedia("(pointer: fine)").matches;

  const toggleMenu = () => {
    const button = document.getElementById("mobile-menu-btn");
    const menu = document.getElementById("mobile-menu");
    if (!button || !menu) return;
    button.addEventListener("click", () => menu.classList.toggle("hidden"));
    menu.querySelectorAll("a").forEach((link) => {
      link.addEventListener("click", () => menu.classList.add("hidden"));
    });
  };

  const setupCursor = () => {
    if (!cursor || !finePointer) return;
    let x = 0;
    let y = 0;
    document.addEventListener(
      "mousemove",
      (event) => {
        x = event.clientX;
        y = event.clientY;
        cursor.style.transform = `translate(${x}px, ${y}px)`;
      },
      { passive: true }
    );
    document.querySelectorAll("a, button, summary, [data-cursor-hover], [role='button']").forEach((element) => {
      element.addEventListener("mouseenter", () => cursor.classList.add("hovering"));
      element.addEventListener("mouseleave", () => cursor.classList.remove("hovering"));
      element.addEventListener("focus", () => cursor.classList.add("hovering"));
      element.addEventListener("blur", () => cursor.classList.remove("hovering"));
    });
  };

  const setupReveal = () => {
    const revealElements = document.querySelectorAll(".reveal");
    if (!revealElements.length) return;
    const observer = new IntersectionObserver(
      (entries) => {
        entries.forEach((entry) => {
          if (entry.isIntersecting) entry.target.classList.add("visible");
        });
      },
      { threshold: 0.1, rootMargin: "0px 0px -40px 0px" }
    );
    revealElements.forEach((element) => observer.observe(element));
  };

  const revealAnchorTarget = () => {
    if (!window.location.hash) return;
    const target = document.getElementById(window.location.hash.slice(1));
    if (!target) return;
    target.classList.add("visible");
    target.querySelectorAll(".reveal").forEach((element) => element.classList.add("visible"));
    requestAnimationFrame(() => {
      target.scrollIntoView({ behavior: "smooth", block: "start" });
    });
  };

  toggleMenu();
  setupCursor();
  setupReveal();
  revealAnchorTarget();
})();
