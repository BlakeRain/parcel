class CheckboxGroup extends HTMLElement {
  private _checkbox: HTMLInputElement | null = null;
  private _dirty: boolean = true;
  private _hasAnyChecked: boolean = false;
  private _hasAllChecked: boolean = false;
  private _changed: ((event: CustomEvent) => void) | null = null;
  private _onCheckboxClick: (() => void) | null = null;
  public lastChecked: GroupedCheckbox | null = null;

  static get observedAttributes() {
    return ["id"];
  }

  constructor() {
    super();
  }

  get id() {
    return this.getAttribute("id");
  }

  get hasAnyChecked() {
    this.fetchCheckboxStates();
    return this._hasAnyChecked;
  }

  get hasAllChecked() {
    this.fetchCheckboxStates();
    return this._hasAllChecked;
  }

  connectedCallback() {
    this._checkbox = document.createElement("input");
    this._checkbox.type = "checkbox";
    this._checkbox.style.margin = "0px";

    // Store the click handler so we can remove it in disconnectedCallback
    this._onCheckboxClick = () => {
      const checked = this._checkbox!.checked;
      this.getChildCheckboxes().forEach((checkbox) => {
        checkbox.checked = checked;
      });

      this._dirty = false;
      this._hasAllChecked = checked;
      this._hasAnyChecked = checked;
      this.dispatchChangedEvent();
    };

    this._checkbox.addEventListener("click", this._onCheckboxClick);

    if (!this.shadowRoot) {
      this.attachShadow({ mode: "open" });
    }

    this.shadowRoot!.appendChild(this._checkbox);

    if (this.hasAttribute("onchanged")) {
      const handler_script = this.getAttribute("onchanged");
      this._changed = new Function("event", handler_script).bind(this) as (event: CustomEvent) => void;
      this.addEventListener("changed", this._changed);
    }
  }

  disconnectedCallback() {
    if (this._checkbox && this._onCheckboxClick) {
      this._checkbox.removeEventListener("click", this._onCheckboxClick);
      this._checkbox.remove();
      this._checkbox = null;
      this._onCheckboxClick = null;
    }

    if (this._changed) {
      this.removeEventListener("changed", this._changed);
      this._changed = null;
    }
  }

  markDirty() {
    this._dirty = true;
  }

  fetchCheckboxStates() {
    if (!this._dirty) {
      return;
    }

    let checked = 0,
      total = 0;
    this.getChildCheckboxes().forEach((checkbox) => {
      if (checkbox.checked) {
        checked++;
      }

      total++;
    });

    this._hasAnyChecked = total > 0 && checked > 0;
    this._hasAllChecked = total > 0 && checked === total;
    this._dirty = false;
  }

  updateCheckbox() {
    this.fetchCheckboxStates();
    if (this._checkbox) {
      this._checkbox.checked = this._hasAllChecked;
      this._checkbox.indeterminate = this._hasAnyChecked;
    }
    this.dispatchChangedEvent();
  }

  dispatchChangedEvent() {
    const event = new CustomEvent("changed", {
      detail: {
        all: this._hasAllChecked,
        any: this._hasAnyChecked,
      },
    });

    this.dispatchEvent(event);
  }

  getChildCheckboxes(): GroupedCheckbox[] {
    return Array.from(
      document.querySelectorAll<GroupedCheckbox>(
        `parcel-grouped-checkbox[group='${this.id}']`,
      ),
    );
  }
}

class GroupedCheckbox extends HTMLElement {
  private _checkbox: HTMLInputElement | null = null;

  get checked() {
    return this._checkbox && this._checkbox.checked;
  }

  set checked(value) {
    if (this._checkbox) {
      this._checkbox.checked = value;
    }
  }

  connectedCallback() {
    const group_name = this.getAttribute("group");
    if (!group_name) {
      throw new Error("GroupedCheckbox requires a 'group' attribute");
    }

    const group = document.getElementById(group_name);
    if (!group) {
      throw new Error(`GroupedCheckbox group '${group_name}' not found`);
    }

    if (!(group instanceof CheckboxGroup)) {
      throw new Error(
        `GroupedCheckbox group '${group_name}' is not a CheckboxGroup`,
      );
    }

    this._checkbox = this.attachCheckbox();
    this._checkbox.type = "checkbox";
    this._checkbox.name = this.getAttribute("name");
    this._checkbox.value = this.getAttribute("value");
    this._checkbox.checked = !!this.getAttribute("checked");

    this._checkbox.addEventListener("click", (event) => {
      if (event.shiftKey) {
        if (group.lastChecked) {
          const children = group.getChildCheckboxes();
          const index = children.indexOf(this);
          const last = children.indexOf(group.lastChecked);

          const checked = group.lastChecked.checked;
          const start = Math.min(index, last);
          const end = Math.max(index, last);

          for (let i = start; i <= end; ++i) {
            children[i].checked = checked;
          }
        }
      }

      group.lastChecked = this;
      group.markDirty();
      group.updateCheckbox();
    });

    group.markDirty();
    group.updateCheckbox();
  }

  disconnectedCallback() {
    if (this._checkbox) {
      this.removeChild(this._checkbox);
      this._checkbox = null;
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
