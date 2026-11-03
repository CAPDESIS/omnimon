import { describe, expect, it } from "vitest";
import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import es from "../../locales/es.json";

// High-confidence misspellings in Spanish copy. In this catalog the noun
// reading dominates; a legitimate verb use needs its own explicit exception.
const BANNED = [
  "configuracion",
  "contrasena",
  "sesion",
  "informacion",
  "notificacion",
  "conexion",
  "energia",
  "pagina",
  "numero",
  "codigo",
  "telefono",
  "tecnica",
  "tecnico",
  "grafica",
  "graficas",
  "grafico",
  "accion",
  "seccion",
  "funcion",
  "ultimo",
  "ultima",
  "maximo",
  "minimo",
  "tambien",
  "despues",
  "aqui",
  "alli",
  "asi",
  "mas", // "mas" without accent only means the archaic conjunction "but"
  "ningun",
  "algun",
  "dia",
  "dias",
  "ano",
  "anos",
  "espanol",
  "limite",
  "analisis",
  "basico",
  "musica",
  "pais",
  "credito",
  "exito",
  "articulo",
  "catalogo",
  "titulo",
  "imagenes",
  "version",
  "revision",
];

// Known-good exceptions: identifier-looking values and verb forms that are
// spelled correctly without an accent in this catalog.
const EXCEPTIONS = new Set([
  // table.energy column header context is covered by pins below; the verb
  // "grafica" (graficar, 3rd person) in networkTrafficBody is correct.
  "Traffic grafica throughput de entrada y salida a lo largo del tiempo para detectar picos o actividad sostenida.",
]);

function collectStrings(node: unknown, out: string[]): void {
  if (typeof node === "string") {
    out.push(node);
    return;
  }
  if (Array.isArray(node)) {
    for (const item of node) collectStrings(item, out);
    return;
  }
  if (node && typeof node === "object") {
    for (const value of Object.values(node)) collectStrings(value, out);
  }
}

function visibleText(value: string): string {
  return value.replace(/\{[^}]*\}/g, " ").replace(/https?:\/\/\S+/g, " ");
}

function findBanned(value: string): string[] {
  const hits: string[] = [];
  const lower = ` ${visibleText(value).toLowerCase()} `;
  for (const word of BANNED) {
    const pattern = new RegExp(`(?<![a-záéíóúñü])${word}(?![a-záéíóúñü])`);
    if (pattern.test(lower)) hits.push(word);
  }
  return hits;
}

describe("spanish orthography", () => {
  it("keeps accents in every es.json value", () => {
    const values: string[] = [];
    collectStrings(es, values);
    expect(values.length).toBeGreaterThan(100);
    const failures: string[] = [];
    for (const value of values) {
      if (EXCEPTIONS.has(value)) continue;
      for (const word of findBanned(value)) {
        failures.push(`[${word}] ${value.slice(0, 100)}`);
      }
    }
    expect(failures).toEqual([]);
  });

  it("pins the fixed energy and empty-state strings", () => {
    const table = (es as Record<string, Record<string, string>>).table;
    expect(table.energy).toBe("Energía");
    expect(table.sortByEnergy).toBe("Ordenar por energía");
    expect(table.emptyDesc).toBe("Ningún proceso coincide con los filtros actuales.");
  });

  it("keeps accents in the network-alert AI prompt", () => {
    const here = dirname(fileURLToPath(import.meta.url));
    const alerts = readFileSync(resolve(here, "../../stores/alerts.ts"), "utf-8");
    expect(alerts).toContain("explícala en español");
    expect(alerts).not.toContain("explicala en espanol");
  });
});
