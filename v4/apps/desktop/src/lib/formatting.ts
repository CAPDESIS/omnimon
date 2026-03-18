/** Formats bytes/sec into human-readable network speed (B/s, KB/s, MB/s). */
export function formatNetworkRate(bytesPerSec: number): string {
  if (bytesPerSec < 1024) return `${bytesPerSec.toFixed(0)} B/s`;
  if (bytesPerSec < 1_048_576) return `${(bytesPerSec / 1024).toFixed(1)} KB/s`;
  return `${(bytesPerSec / 1_048_576).toFixed(1)} MB/s`;
}
