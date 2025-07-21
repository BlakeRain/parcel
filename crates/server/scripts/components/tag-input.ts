const VALID_TAG_RE = /^[ a-zA-Z0-9-]+$/;

class ParcelTagInput extends HTMLElement {
  #tags: string[] = [];
  #tagContainer: HTMLElement;
  #tagInput: HTMLInputElement;
  #addButton: HTMLButtonElement;
  #internals = this.attachInternals();

  static formAssociated = true;

  connectedCallback() {
    this.classList.add("tag-input");

    this.#tags = this.#getTags();
    this.#updateFormValue();

    this.#tagContainer = document.createElement("div");
    this.#tagContainer.classList.add("tag-container");

    this.#tags.forEach((tag, index) => {
      const tagElement = this.#createTagElement(tag, index);
      this.#tagContainer.appendChild(tagElement);
    });

    const inputContainer = document.createElement("div");
    inputContainer.classList.add("input-container");

    this.#creatAddButton();
    this.#createTagInput();
    inputContainer.append(this.#tagInput, this.#addButton);
    this.append(this.#tagContainer, inputContainer);
  }

  #createTagElement(tag: string, index: number): HTMLElement {
    const tagElement = document.createElement("div");
    tagElement.classList.add("tag", "large");

    const nameElement = document.createElement("span");
    nameElement.classList.add("name");
    nameElement.textContent = tag;

    const button = document.createElement("button");
    button.type = "button";
    button.classList.add("icon-x");
    button.addEventListener("click", () => {
      this.#tags.splice(index, 1);
      tagElement.remove();
      this.#updateFormValue();
    });

    tagElement.append(nameElement, button);
    return tagElement;
  }

  #createTagInput() {
    this.#tagInput = document.createElement("input");
    this.#tagInput.type = "text";
    this.#tagInput.placeholder = "Add a tag, press Enter";
    this.#tagInput.pattern = VALID_TAG_RE.source;
    this.#tagInput.classList.add("field", "grow");

    const datalist = this.querySelector("datalist");
    this.#tagInput.setAttribute("list", datalist ? datalist.id : "");

    this.#tagInput.addEventListener("input", () => {
      this.#validateInput();
    });

    this.#tagInput.addEventListener("keydown", (event) => {
      if (event.key === "Enter") {
        event.preventDefault();

        if (this.#isValidInput()) {
          this.#addTag();
        }
      } else {
        this.#validateInput();
      }
    });
  }

  #creatAddButton() {
    this.#addButton = document.createElement("button");
    this.#addButton.type = "button";
    this.#addButton.classList.add("button");
    this.#addButton.title = "Add tag";
    this.#addButton.disabled = true;
    this.#addButton.addEventListener("click", () => {
      if (this.#isValidInput()) {
        this.#addTag();
      }
    });

    const addIcon = document.createElement("span");
    addIcon.classList.add("icon-plus");

    const addText = document.createTextNode(" Add");
    this.#addButton.append(addIcon, addText);
  }

  #getInputTag(): string {
    return this.#tagInput.value.trim().replace(/\s+/g, " ");
  }

  #isValidInput() {
    const tag = this.#getInputTag();
    if (tag.length === 0) {
      return false;
    }

    if (!VALID_TAG_RE.test(tag)) {
      return false;
    }

    return true;
  }

  #validateInput() {
    const valid = this.#isValidInput();
    this.#tagInput.classList.toggle("invalid", !valid);
    this.#addButton.disabled = !valid;
  }

  #addTag() {
    if (!this.#isValidInput()) {
      return;
    }

    const tag = this.#getInputTag();
    this.#tags.push(tag);
    const tagElement = this.#createTagElement(tag, this.#tags.length - 1);
    this.#tagContainer.appendChild(tagElement);
    this.#tagInput.value = "";
    this.#tagInput.classList.remove("invalid");
    this.#addButton.disabled = true;
    this.#updateFormValue();
  }

  #getTags() {
    const value = this.getAttribute("value") || "";
    return value
      .split(",")
      .map((tag) => tag.trim())
      .filter((tag) => tag);
  }

  #updateFormValue() {
    const name = this.getAttribute("name") || "tags";

    const formData = new FormData();
    this.#tags.forEach((tag) => {
      formData.append(name, tag);
    });

    this.#internals.setFormValue(formData);
  }
}

export function register() {
  customElements.define("parcel-tag-input", ParcelTagInput);
}
