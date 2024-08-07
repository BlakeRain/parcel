const CHECKBOX_GROUPS = {};

class CheckboxGroup extends HTMLElement {
  static get observedAttributes() {
    return ["name"];
  }

  constructor() {
    super();
    this.checkbox = null;
    this.checkboxes = [];
    this.last_checked = null;
  }

  connectedCallback() {
    let name = this.getAttribute("name");
    if (!name) {
      throw Error("CheckboxGroup requires a name attribute");
    }

    this.checkbox = document.createElement("input");
    this.checkbox.type = "checkbox";
    this.checkbox.name = name;

    this.checkbox.addEventListener("click", () => {
      this.checkboxes.forEach((checkbox) => {
        checkbox.checkbox.checked = this.checkbox.checked;
      });

      this.dispatchChangedEvent();
    });

    const shadow = this.attachShadow({ mode: "open" });
    shadow.appendChild(this.checkbox);

    CHECKBOX_GROUPS[name] = this;
  }

  registerCheckbox(checkbox) {
    const index = this.checkboxes.length;
    this.checkboxes.push(checkbox);
    return index;
  }

  updateCheckbox() {
    const checked = this.checkboxes.filter(
      (checkbox) => checkbox.checkbox.checked,
    );
    this.checkbox.checked = checked.length === this.checkboxes.length;
    this.checkbox.indeterminate =
      checked.length > 0 && checked.length < this.checkboxes.length;
    this.dispatchChangedEvent();
  }

  dispatchChangedEvent() {
    const checked = this.checkboxes.filter(
      (checkbox) => checkbox.checkbox.checked,
    );

    const event = new CustomEvent("changed", {
      detail: {
        all: checked.length == this.checkboxes.length,
        any: checked.length > 0,
      },
    });

    this.dispatchEvent(event);
  }
}

class GroupedCheckbox extends HTMLElement {
  static get observedAttributes() {
    return ["group", "name"];
  }

  constructor() {
    super();
    this.index = -1;
    this.checkbox = null;
  }

  connectedCallback() {
    const group_name = this.getAttribute("group");
    if (!group_name) {
      throw Error("GroupedCheckbox requires a group attribute");
    }

    const group = CHECKBOX_GROUPS[group_name];
    if (!group) {
      throw Error(`GroupedCheckbox group '${group_name}' not found`);
    }

    this.index = group.registerCheckbox(this);

    this.checkbox = document.createElement("input");
    this.checkbox.type = "checkbox";
    this.checkbox.name = this.getAttribute("name");
    this.checkbox.value = this.getAttribute("value");

    this.checkbox.addEventListener("click", (event) => {
      if (event.shiftKey) {
        if (group.last_checked) {
          const checked = group.last_checked.checkbox.checked;
          const start = Math.min(this.index, group.last_checked.index);
          const end = Math.max(this.index, group.last_checked.index);
          for (let i = start; i <= end; ++i) {
            group.checkboxes[i].checkbox.checked = checked;
          }
        }
      }

      group.last_checked = this;
      group.updateCheckbox();
    });

    this.appendChild(this.checkbox);

    // const shadow = this.attachShadow({ mode: "open" });
    // shadow.appendChild(this.checkbox);
  }
}

export function register() {
  customElements.define("parcel-checkbox-group", CheckboxGroup);
  customElements.define("parcel-grouped-checkbox", GroupedCheckbox);
}
