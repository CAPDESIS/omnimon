const fs = require('fs');
const file = 'src/components/SystemMetricModal.svelte';
let content = fs.readFileSync(file, 'utf8');

// Add the helper function right before the sparklinePath function
const addFunc = `
  function getSparklineColor(m: string, series: {value: number}[]): string {
    if (m !== "cpu" && m !== "ram") return "var(--accent)";
    if (series.length === 0) return "var(--accent)";
    const latest = series[series.length - 1].value;
    if (latest >= 80) return "var(--danger)";
    if (latest >= 60) return "var(--yellow)";
    return "var(--green)";
  }

  function sparklinePath(`;

content = content.replace('  function sparklinePath(', addFunc);

// Update the path stroke
content = content.replace(
  '<path d={sparklinePath(activeSeries)} fill="none" stroke="var(--accent)" stroke-width="1.5" />',
  '<path d={sparklinePath(activeSeries)} fill="none" stroke={getSparklineColor(metric, activeSeries)} stroke-width="1.5" />'
);

fs.writeFileSync(file, content);
