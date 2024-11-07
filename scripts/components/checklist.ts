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

  get hasAnyChecked() {
    return this.checkboxes.some((checkbox) => checkbox.checked);
  }

  get hasAllChecked() {
    return this.checkboxes.every((checkbox) => checkbox.checked);
  }

  connectedCallback() {
    let name = this.getAttribute("name");
    if (!name) {
      throw Error("CheckboxGroup requires a name attribute");
    }

    this.checkbox = document.createElement("input");
    this.checkbox.type = "checkbox";
    this.checkbox.name = name;
    this.checkbox.style.margin = "0px";

    this.checkbox.addEventListener("click", () => {
      this.checkboxes.forEach((checkbox) => {
        checkbox.checked = this.checkbox.checked;
      });

      this.dispatchChangedEvent();
    });

    const shadow = this.attachShadow({ mode: "open" });
    shadow.appendChild(this.checkbox);

    if (this.hasAttribute("onchanged")) {
      const handler_script = this.getAttribute("onchanged");
      const func = new Function("event", handler_script).bind(this);
      this.addEventListener("changed", func);
    }

    if (CHECKBOX_GROUPS[name]) {
      throw new Error(`CheckboxGroup '${name}' already exists`);
    }

    CHECKBOX_GROUPS[name] = this;
  }

  disconnectedCallback() {
    delete CHECKBOX_GROUPS[this.getAttribute("name")];
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
  private group: CheckboxGroup = null;
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
    const group_name = this.getAttribute("group");
    if (!group_name) {
      throw new Error("GroupedCheckbox requires a 'group' attribute");
    }

    this.group = CHECKBOX_GROUPS[group_name];
    if (!this.group) {
      throw new Error(`GroupedCheckbox group '${group_name}' not found`);
    }

    this.index = this.group.registerCheckbox(this);
    this.checkbox = this.attachCheckbox();
    this.checkbox.type = "checkbox";
    this.checkbox.name = this.getAttribute("name");
    this.checkbox.value = this.getAttribute("value");
    this.checkbox.checked = !!this.getAttribute("checked");

    this.checkbox.addEventListener("click", (event) => {
      if (event.shiftKey) {
        if (this.group.lastChecked) {
          const checked = this.group.lastChecked.checked;
          const start = Math.min(this.index, this.group.lastChecked.index);
          const end = Math.max(this.index, this.group.lastChecked.index);
          for (let i = start; i <= end; ++i) {
            this.group.checkboxes[i].checked = checked;
          }
        }
      }

      this.group.lastChecked = this;
      this.group.updateCheckbox();
    });
  }

  disconnectedCallback() {
    if (this.checkbox) {
      this.removeChild(this.checkbox);
      this.checkbox = null;
      this.group.unregisterCheckbox(this.index);
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
