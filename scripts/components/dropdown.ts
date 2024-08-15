const DROPDOWN_ELEMENTS: Set<Dropdown> = new Set();

function hideDropdowns() {
  for (const dropdown of DROPDOWN_ELEMENTS) {
    dropdown.hide();
  }
}

class Dropdown extends HTMLElement {
  private open: boolean = false;
  private button: HTMLSpanElement;
  private dropdown: HTMLDivElement;

  static BUTTON_TEMPLATE = document.createElement("template");
  static DROPDOWN_TEMPLATE = document.createElement("template");

  static {
    this.BUTTON_TEMPLATE.innerHTML = `
      <span class=></span>
    `.trim();

    this.DROPDOWN_TEMPLATE.innerHTML = `
      <div class="z-10 hidden"></div>
    `.trim();
  }

  connectedCallback() {
    this.className = "inline-block relative";

    this.dropdown = document.createElement("div");
    this.dropdown.className =
      "has-triangle triangle-tr absolute right-[-0.3rem] top-[1.9rem] hidden z-10 w-44 rounded-lg shadow bg-neutral-100 dark:bg-gray-700";

    while (this.firstChild) {
      this.dropdown.appendChild(this.firstChild);
    }

    this.button = document.createElement("div");
    this.button.className =
      "flex items-center justify-center cursor-pointer w-6 h-6 rounded-full hover:bg-neutral-100 dark:hover:bg-gray-700 text-neutral-800 hover:text-neutral-900 dark:text-neutral-300 dark:hover:text-neutral-100";

    const icon = document.createElement("span");
    icon.className = "icon-ellipsis";
    this.button.appendChild(icon);

    this.button.addEventListener("click", (event) => {
      if (this.open) {
        this.dropdown.classList.add("hidden");
      } else {
        hideDropdowns();
        this.dropdown.classList.remove("hidden");
      }

      this.open = !this.open;
      event.stopPropagation();
    });

    this.appendChild(this.button);
    this.appendChild(this.dropdown);
    DROPDOWN_ELEMENTS.add(this);
  }

  disconnectedCallback() {
    this.removeChild(this.dropdown);
    this.removeChild(this.button);

    while (this.dropdown.firstChild) {
      this.appendChild(this.dropdown.firstChild);
    }

    this.dropdown = null;
    this.button = null;
    DROPDOWN_ELEMENTS.delete(this);
  }

  hide() {
    this.dropdown.classList.add("hidden");
    this.open = false;
  }
}

export function register() {
  customElements.define("parcel-dropdown", Dropdown);

  window.addEventListener("click", () => {
    hideDropdowns();
  });
}
