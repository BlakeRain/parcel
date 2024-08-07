export function formatBytes(bytes, digits) {
  const units = ["B", "KB", "MB", "GB", "TB"];
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
