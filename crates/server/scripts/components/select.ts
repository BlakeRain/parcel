import { createContext, FunctionComponent } from "preact";
import { useContext, useEffect, useMemo, useReducer } from "preact/hooks";
import { html } from "htm/preact";
import { default as registerElement } from "preact-custom-element";
import { loadFromScript } from "../utils";

interface Option {
  value: string;
  label: string;
}

const OptionsContext = createContext<Option[]>([]);

const WithOptions: FunctionComponent<{ selector: string }> = ({
  selector,
  children,
}) => {
  const options = useMemo(() => loadFromScript<Option[]>(selector), [selector]);

  return html`
    <${OptionsContext.Provider} value=${options}>${children}</${OptionsContext.Provider}>
  `;
};

interface State {
  open: boolean;
  checked: Set<string>;
}

type StateAction =
  | { type: "open" }
  | { type: "close" }
  | { type: "clear" }
  | { type: "check"; value: string }
  | { type: "uncheck"; value: string }
  | { type: "replace"; checked: Set<string> };

function createInitialState(selector?: string): State {
  const values = selector ? loadFromScript<string[]>(selector) : [];
  const checked = new Set<string>(values);

  return {
    open: false,
    checked,
  };
}

function reduceState(state: State, action: StateAction): State {
  switch (action.type) {
    case "open":
      return { ...state, open: true };
    case "close":
      return { ...state, open: false };
    case "clear":
      return { ...state, checked: new Set() };
    case "check":
      state.checked.add(action.value);
      return { ...state, checked: new Set(state.checked) };
    case "uncheck":
      state.checked.delete(action.value);
      return { ...state, checked: new Set(state.checked) };
    case "replace":
      return { ...state, checked: action.checked };
    default:
      throw new Error(`Invalid action type: ${action}`);
  }
}

function stopPropagation(event: Event) {
  event.stopPropagation();
}

const StateContext = createContext<{ state: State; dispatch: (action: StateAction) => void }>({
  state: createInitialState(),
  dispatch: () => {
    throw new Error("attempt to use dispatch without state context");
  },
});

const WithState: FunctionComponent<{ selector?: string }> = ({
  selector,
  children,
}) => {
  const [state, dispatch] = useReducer(
    reduceState,
    createInitialState(selector),
  );
  return html`
    <${StateContext.Provider} value=${{ state, dispatch }}>
      ${children}
    </${StateContext.Provider}>
  `;
};

const ParcelSelectOption: FunctionComponent<{
  name: string;
  option: Option;
}> = ({ name, option }) => {
  const { state, dispatch } = useContext(StateContext);

  const onCheckboxChange = (event: InputEvent) => {
    const target = event.target as HTMLInputElement;
    if (target.checked) {
      dispatch({ type: "check", value: option.value });
    } else {
      dispatch({ type: "uncheck", value: option.value });
    }
  };

  return html`
    <label class="parcel-select-option" onclick=${stopPropagation}>
      <input
        type="checkbox"
        name=${name}
        value=${option.value}
        checked=${state.checked.has(option.value)}
        onclick=${stopPropagation}
        onchange=${onCheckboxChange}
      />
      <span>${option.label}</span>
    </label>
  `;
};

const ParcelSelectDropdown: FunctionComponent<{ name: string }> = ({
  name,
}) => {
  const options = useContext(OptionsContext);
  const { state, dispatch } = useContext(StateContext);

  const onSelectAllClick = (event: MouseEvent) => {
    event.stopPropagation();
    event.preventDefault();
    dispatch({
      type: "replace",
      checked: new Set(options.map((option) => option.value)),
    });
  };

  const onClearSelectionClick = (event: MouseEvent) => {
    event.stopPropagation();
    event.preventDefault();
    dispatch({ type: "clear" });
  };

  const optionElements = options.map(
    (option) => html`<${ParcelSelectOption} name=${name} option=${option} />`,
  );

  return html`
    <div class="parcel-select-options">
      <div class="flex flex-row justify-between mx-2 py-2">
        <a href="#" onclick=${onSelectAllClick}>Select all</a>
        <a
          href="#"
          disabled=${state.checked.size === 0}
          onclick=${onClearSelectionClick}
          >Clear selection</a
        >
      </div>
      ${optionElements}
    </div>
  `;
};

const ParcelSelectInfo: FunctionComponent<{ placeholder?: string }> = ({
  placeholder,
}) => {
  const options = useContext(OptionsContext);
  const { state } = useContext(StateContext);

  const names = options
    .filter((option) => state.checked.has(option.value))
    .map((option) => option.label)
    .join(", ");

  return html`
    <div class="grow noselect">
      <span
        >${state.checked.size == 0
          ? placeholder || "No selection"
          : `${state.checked.size} selected:`}</span
      >
      <span class="text-ellipsis ml-1 italic text-gray-500 dark:text-gray-500"
        >${names}</span
      >
    </div>
  `;
};

const ParcelSelect: FunctionComponent<{
  name: string;
  class?: string;
  placeholder?: string;
}> = (props) => {
  const { state, dispatch } = useContext(StateContext);

  const onOuterClick = () => {
    dispatch({ type: state.open ? "close" : "open" });
  };

  useEffect(() => {
    const onWindowClick = (event: MouseEvent) => {
      if (!(event.target instanceof HTMLElement)) {
        return;
      }

      if (!event.target.closest(".parcel-select")) {
        dispatch({ type: "close" });
      }
    };

    window.addEventListener("click", onWindowClick);

    return () => {
      window.removeEventListener("click", onWindowClick);
    };
  }, []);

  return html`
    <div
      class="parcel-select ${props.class} ${state.open ? "open" : ""}"
      onclick=${onOuterClick}
    >
      <${ParcelSelectInfo} placeholder=${props.placeholder} />
      <${ParcelSelectDropdown} name=${props.name} />
    </div>
  `;
};

const ParcelSelectOuter: FunctionComponent<{
  name: string;
  options: string;
  values?: string;
  class?: string;
  placeholder?: string;
}> = (props) => {
  return html`
    <${WithOptions} selector=${props.options}>
      <${WithState} selector=${props.values}>
        <${ParcelSelect} name=${props.name} class=${props.class} placeholder=${props.placeholder} />
      </${WithState}>
    </${WithOptions}>
  `;
};

export function register() {
  registerElement(ParcelSelectOuter, "parcel-select", [
    "name",
    "class",
    "options",
  ]);
}
