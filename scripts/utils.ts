export function loadFromScript<T>(selector: string): T {
  const element = document.querySelector<HTMLScriptElement>(selector);
  if (!element || element.tagName !== "SCRIPT") {
    throw new Error(
      `Selector '${selector}' did not match any <script> elements`,
    );
  }

  try {
    return JSON.parse(element.textContent);
  } catch (error) {
    throw new Error(`Failed to parse JSON from <script> element: ${error}`);
  }
}
