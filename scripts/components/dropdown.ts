const DROPDOWN_ELEMENTS: Set<Dropdown> = new Set();

function hideDropdowns() {
  for (const dropdown of DROPDOWN_ELEMENTS) {
    dropdown.hide();
  }
}

class Dropdown extends HTMLElement {
  private open: boolean = false;
  private button: HTMLDivElement;
  private dropdown: HTMLDivElement;

  connectedCallback() {
    this.className = "dropdown";

    if (
      this.firstElementChild &&
      this.firstElementChild.nodeName === "DIV" &&
      this.firstElementChild.classList.contains("dropdown-button")
    ) {
      this.button = this.firstElementChild as HTMLDivElement;
      this.dropdown = this.lastElementChild as HTMLDivElement;
    } else {
      this.dropdown = document.createElement("div");
      this.dropdown.className = "dropdown-menu";

      while (this.firstChild) {
        this.dropdown.appendChild(this.firstChild);
      }

      this.button = document.createElement("div");
      this.button.className = "dropdown-button";

      const icon = document.createElement("span");
      icon.className = "icon-menu";
      this.button.appendChild(icon);
      this.appendChild(this.button);
      this.appendChild(this.dropdown);
    }

    // Attach the event handler, even if we didn't create the button.
    this.button.addEventListener("click", (event) => {
      if (this.open) {
        this.dropdown.classList.remove("open");
      } else {
        hideDropdowns();
        this.dropdown.classList.add("open");
      }

      this.open = !this.open;
      event.stopPropagation();
    });

    DROPDOWN_ELEMENTS.add(this);
  }

  disconnectedCallback() {
    this.dropdown = null;
    this.button = null;
    DROPDOWN_ELEMENTS.delete(this);
  }

  hide() {
    this.dropdown.classList.remove("open");
    this.open = false;
  }
}

export function register() {
  customElements.define("parcel-dropdown", Dropdown);

  window.addEventListener("click", () => {
    hideDropdowns();
  });
}
