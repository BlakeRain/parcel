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
        checkbox.checkbox.checked = this.checkbox.checked;
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
  public index: number = -1;
  public checkbox: HTMLInputElement = null;
  private internals: ElementInternals;

  static formAssociated = true;

  static get observedAttributes() {
    return ["group", "name"];
  }

  get checked() {
    return this.checkbox.checked;
  }

  set checked(v) {
    this.checkbox.checked = v;
  }

  get value() {
    return this.checkbox.checked ? this.checkbox.value : null;
  }

  set value(v) {
    this.checkbox.value = v;
  }

  get form() {
    return this.internals.form;
  }

  get name() {
    return this.getAttribute("name");
  }

  get type() {
    return this.localName;
  }

  get validity() {
    return this.internals.validity;
  }

  get validationMessage() {
    return this.internals.validationMessage;
  }

  get willValidate() {
    return this.internals.willValidate;
  }

  checkValidity() {
    this.internals.checkValidity();
  }

  reportValidity() {
    this.internals.reportValidity();
  }

  constructor() {
    super();
    this.internals = this.attachInternals();
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
    this.checkbox.checked = !!this.getAttribute("checked");

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

    const shadow = this.attachShadow({ mode: "open" });
    shadow.appendChild(this.checkbox);
  }
}

export function register() {
  customElements.define("parcel-checkbox-group", CheckboxGroup);
  customElements.define("parcel-grouped-checkbox", GroupedCheckbox);
}
