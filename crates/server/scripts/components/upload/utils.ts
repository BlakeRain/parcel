export function formatBytes(bytes: number, digits?: number): string {
  const units = ["bytes", "KB", "MB", "GB", "TB"];
  let unit = 0;

  while (bytes >= 1024 && unit < units.length - 1) {
    bytes /= 1024;
    ++unit;
  }

  let actualDigits = typeof digits === "number" ? digits : 2;

  return (
    new Intl.NumberFormat("en-US", {
      minimumFractionDigits: actualDigits,
      maximumFractionDigits: actualDigits,
    }).format(bytes) +
    " " +
    units[unit]
  );
}
