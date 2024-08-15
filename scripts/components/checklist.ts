const CHECKBOX_GROUPS = {};

class CheckboxGroup extends HTMLElement {
  public checkbox: HTMLInputElement = null;
  public checkboxes: GroupedCheckbox[] = [];
  public lastChecked: GroupedCheckbox | null = null;

  static get observedAttributes() {
    return ["name"];
  }

  constructor() {
    super();
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
        checkbox.checked = this.checkbox.checked;
      });

      this.dispatchChangedEvent();
    });

    const shadow = this.attachShadow({ mode: "open" });
    shadow.appendChild(this.checkbox);

    CHECKBOX_GROUPS[name] = this;
  }

  registerCheckbox(checkbox: GroupedCheckbox) {
    const index = this.checkboxes.length;
    this.checkboxes.push(checkbox);
    return index;
  }

  unregisterCheckbox(index: number) {
    this.checkboxes.splice(index, 1);
  }

  updateCheckbox() {
    const checked = this.checkboxes.filter((checkbox) => checkbox.checked);
    this.checkbox.checked = checked.length === this.checkboxes.length;
    this.checkbox.indeterminate =
      checked.length > 0 && checked.length < this.checkboxes.length;
    this.dispatchChangedEvent();
  }

  dispatchChangedEvent() {
    const checked = this.checkboxes.filter((checkbox) => checkbox.checked);

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
  private index = -1;
  private group_name: string = null;
  private checkbox: HTMLInputElement = null;

  get checked() {
    return this.checkbox && this.checkbox.checked;
  }

  set checked(value) {
    if (this.checkbox) {
      this.checkbox.checked = value;
    }
  }

  connectedCallback() {
    this.group_name = this.getAttribute("group");
    if (!this.group_name) {
      throw new Error("GroupedCheckbox requires a 'group' attribute");
    }

    const group = CHECKBOX_GROUPS[this.group_name];
    if (!group) {
      throw new Error(`GroupedCheckbox group '${this.group_name}' not found`);
    }

    this.index = group.registerCheckbox(this);
    this.checkbox = this.attachCheckbox();
    this.checkbox.type = "checkbox";
    this.checkbox.name = this.getAttribute("name");
    this.checkbox.value = this.getAttribute("value");
    this.checkbox.checked = !!this.getAttribute("checked");

    this.checkbox.addEventListener("click", (event) => {
      if (event.shiftKey) {
        if (group.lastChecked) {
          const checked = group.lastChecked.checked;
          const start = Math.min(this.index, group.lastChecked.index);
          const end = Math.max(this.index, group.lastChecked.index);
          for (let i = start; i <= end; ++i) {
            group.checkboxes[i].checked = checked;
          }
        }
      }

      group.lastChecked = this;
      group.updateCheckbox();
    });
  }

  disconnectedCallback() {
    if (this.checkbox) {
      this.removeChild(this.checkbox);
      this.checkbox = null;
      CHECKBOX_GROUPS[this.group_name].unregisterCheckbox(this.index);
    }
  }

  attachCheckbox() {
    let checkbox = null;

    if (this.children.length > 0) {
      checkbox = this.children[0];
      if (checkbox.tagName !== "INPUT") {
        checkbox = null;
      }
    }

    if (!checkbox) {
      checkbox = this.attachNewCheckbox();
    }

    return checkbox;
  }

  attachNewCheckbox() {
    const checkbox = document.createElement("input");
    this.appendChild(checkbox);
    return checkbox;
  }
}

export function register() {
  customElements.define("parcel-checkbox-group", CheckboxGroup);
  customElements.define("parcel-grouped-checkbox", GroupedCheckbox);
}
