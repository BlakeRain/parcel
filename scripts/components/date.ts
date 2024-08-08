class TimeElement extends HTMLElement {
  static get observedAttributes() {
    return ["value"];
  }

  constructor() {
    super();
  }

  connectedCallback() {
    const valueAttribute = this.getAttribute("value");
    if (!valueAttribute) {
      return;
    }

    const value = new Date(valueAttribute);
    const shadow = this.attachShadow({ mode: "open" });
    const span = document.createElement("span");
    span.title = this.formatTitle(value);
    this.formatContent(span, value);
    shadow.appendChild(span);
  }

  formatTitle(_value: Date): string {
    throw new Error("Not implemented");
  }

  formatContent(_span: HTMLSpanElement, _value: Date) {
    throw new Error("Not implemented");
  }
}

const DATE_FORMAT = new Intl.DateTimeFormat(undefined, { dateStyle: "medium" });
const DATE_FORMAT_FULL = new Intl.DateTimeFormat(undefined, {
  dateStyle: "full",
});

const DATETIME_FORMAT = new Intl.DateTimeFormat(undefined, {
  dateStyle: "short",
  timeStyle: "short",
});
const DATETIME_FORMAT_FULL = new Intl.DateTimeFormat(undefined, {
  dateStyle: "full",
  timeStyle: "long",
});

export function register() {
  customElements.define(
    "parcel-date",
    class extends TimeElement {
      formatTitle(value: Date): string {
        return DATE_FORMAT_FULL.format(value);
      }

      formatContent(span: HTMLSpanElement, value: Date) {
        span.textContent = DATE_FORMAT.format(value);
      }
    },
  );

  customElements.define(
    "parcel-datetime",
    class extends TimeElement {
      formatTitle(value: Date): string {
        return DATETIME_FORMAT_FULL.format(value);
      }

      formatContent(span: HTMLSpanElement, value: Date) {
        span.textContent = DATETIME_FORMAT.format(value);
      }
    },
  );
}
