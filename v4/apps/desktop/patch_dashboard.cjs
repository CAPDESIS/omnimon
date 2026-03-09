const fs = require('fs');
const file = 'src/components/SystemDashboard.svelte';
let content = fs.readFileSync(file, 'utf8');

// replace colorForPct to be more useful
content = content.replace(
`  function colorForPct(pct: number): string {
    if (pct >= 80) return "var(--danger)";
    if (pct >= 60) return "var(--yellow)";
    return "var(--green)";
  }`,
`  function colorVarForPct(pct: number): string {
    if (pct >= 80) return "--danger";
    if (pct >= 60) return "--yellow";
    return "--green";
  }

  function colorForPct(pct: number): string {
    return \`var(\${colorVarForPct(pct)})\`;
  }`
);

// fix drawSparkline calls
content = content.replace(
`    if (cpuCanvas) {
      drawSparkline(cpuCanvas, h.map((s) => s.cpuAvg), "--chart-cpu", 100);
    }`,
`    if (cpuCanvas) {
      const avg = h[h.length - 1]?.cpuAvg || 0;
      drawSparkline(cpuCanvas, h.map((s) => s.cpuAvg), colorVarForPct(avg), 100);
    }`
);

content = content.replace(
`    if (ramCanvas) {
      drawSparkline(ramCanvas, h.map((s) => s.ramPct), "--chart-ram", 100);
    }`,
`    if (ramCanvas) {
      const pct = h[h.length - 1]?.ramPct || 0;
      drawSparkline(ramCanvas, h.map((s) => s.ramPct), colorVarForPct(pct), 100);
    }`
);

fs.writeFileSync(file, content);
