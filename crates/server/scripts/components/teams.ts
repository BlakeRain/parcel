import { createContext, FunctionComponent } from "preact";
import { useContext, useMemo, useReducer } from "preact/hooks";
import { html } from "htm/preact";
import { default as registerElement } from "preact-custom-element";
import { loadFromScript } from "../utils";

type TargetId = string;
type TeamPermission = "edit" | "delete" | "config";

type TeamPermissions = {
  [key in TeamPermission]: boolean;
};

interface Target {
  id: TargetId;
  name: string;
}

const TargetsContext = createContext<Target[]>([]);

const WithTargets: FunctionComponent<{ selector: string }> = ({
  selector,
  children,
}) => {
  const targets = useMemo(() => loadFromScript<Target[]>(selector), [selector]);
  targets.sort((a, b) => a.name.localeCompare(b.name));

  return html`<${TargetsContext.Provider} value=${targets}>${children}<//>`;
};

interface State {
  permissions: Map<TargetId, TeamPermissions>;
}

type StateAction =
  | { type: "add"; id: TargetId }
  | { type: "remove"; id: TargetId }
  | { type: "set"; id: TargetId; permission: TeamPermission }
  | { type: "clear"; id: TargetId; permission: TeamPermission };

function createInitialState(selector?: string): State {
  const permissions = selector
    ? loadFromScript<{ [key: TargetId]: TeamPermissions }>(selector)
    : {};
  return { permissions: new Map(Object.entries(permissions)) };
}

function reduceState(state: State, action: StateAction): State {
  switch (action.type) {
    case "add": {
      const permissions = new Map(state.permissions);
      permissions.set(action.id, { edit: false, delete: false, config: false });
      return { permissions };
    }

    case "remove": {
      const permissions = new Map(state.permissions);
      permissions.delete(action.id);
      return { permissions };
    }

    case "set": {
      const permissions = new Map(state.permissions);
      const team = permissions.get(action.id);
      if (team) {
        team[action.permission] = true;
      }
      return { permissions };
    }

    case "clear": {
      const permissions = new Map(state.permissions);
      const team = permissions.get(action.id);
      if (team) {
        team[action.permission] = false;
        if (action.permission === "edit") {
          team.delete = false;
        }
      }
      return { permissions };
    }
  }
}

const StateContext = createContext<{
  state: State;
  dispatch: (action: StateAction) => void;
}>({
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
    createInitialState(selector)
  );

  return html`
    <${StateContext.Provider} value=${{ state, dispatch }}> ${children} <//>
  `;
};

const HiddenField: FunctionComponent<{ name?: string }> = ({ name }) => {
  const { state } = useContext(StateContext);
  return html`
    <input
      type="hidden"
      name="${name}"
      value=${JSON.stringify(Object.fromEntries(state.permissions))}
    />
  `;
};

const TargetRow: FunctionComponent<{
  target: Target;
  enabling: boolean;
  self?: string;
}> = ({ target, enabling, self }) => {
  const { state, dispatch } = useContext(StateContext);
  const permissions = state.permissions.get(target.id);

  const enable = enabling
    ? html`
        <input
          id="target-${target.id}"
          type="checkbox"
          style="margin: 0;"
          checked=${typeof permissions !== "undefined"}
          onChange=${(event: InputEvent) => {
            if ((event.target as HTMLInputElement).checked) {
              dispatch({ type: "add", id: target.id });
            } else {
              dispatch({ type: "remove", id: target.id });
            }
          }}
        />
      `
    : html`<div></div>`;

  return html`
    ${enable}
    <label
      for="target-${target.id}"
      class="${permissions ? "" : "opacity-75 italic"}"
      style="margin: 0;"
      >${target.name}
      ${self && target.id === self
        ? html`<span class="text-primary-400 dark:text-primary-500 ml-1">
            (You)</span
          >`
        : null}
    </label>
    <input
      type="checkbox"
      style="margin: 0;"
      disabled=${!permissions}
      checked=${permissions?.edit}
      onChange=${(event: InputEvent) => {
        if ((event.target as HTMLInputElement).checked) {
          dispatch({ type: "set", id: target.id, permission: "edit" });
        } else {
          dispatch({ type: "clear", id: target.id, permission: "edit" });
        }
      }}
    />
    <input
      type="checkbox"
      style="margin: 0;"
      disabled=${!permissions || !permissions.edit}
      checked=${permissions?.delete}
      onChange=${(event: InputEvent) => {
        if ((event.target as HTMLInputElement).checked) {
          dispatch({ type: "set", id: target.id, permission: "delete" });
        } else {
          dispatch({ type: "clear", id: target.id, permission: "delete" });
        }
      }}
    />
    <input
      type="checkbox"
      style="margin: 0;"
      disabled=${!permissions}
      checked=${permissions?.config}
      onChange=${(event: InputEvent) => {
        if ((event.target as HTMLInputElement).checked) {
          dispatch({ type: "set", id: target.id, permission: "config" });
        } else {
          dispatch({ type: "clear", id: target.id, permission: "config" });
        }
      }}
    />
  `;
};

const TargetRows: FunctionComponent<{ enabling: boolean; self?: string }> = ({
  enabling,
  self,
}) => {
  const targets = useContext(TargetsContext);
  return html`
    ${targets.map(
      (target) =>
        html`<${TargetRow}
          target=${target}
          enabling=${enabling}
          self=${self}
        />`
    )}
  `;
};

const TargetHeadings: FunctionComponent<{ noun: string }> = ({ noun }) => {
  return html`
    <span class="font-medium text-sm text-gray-900 dark:text-white col-start-2"
      >${noun}</span
    >
    <span class="font-medium text-sm text-gray-900 dark:text-white">Edit</span>
    <span class="font-medium text-sm text-gray-900 dark:text-white"
      >Delete</span
    >
    <span class="font-medium text-sm text-gray-900 dark:text-white"
      >Config</span
    >
  `;
};

const TableContainer: FunctionComponent<{ class?: string }> = ({
  children,
  ...props
}) => {
  return html`
    <div
      class="sm:text-sm bg-gray-50 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-lg p-2.5 grid grid-cols-[max-content_1fr_repeat(3,max-content)] items-center gap-1 ${props.class}"
    >
      ${children}
    </div>
  `;
};

const ParcelTeams: FunctionComponent<{
  name: string;
  noun: string;
  class?: string;
  enabling?: boolean;
  self?: string;
  targets?: string;
  permissions?: string;
}> = (props) => {
  return html`
    <${WithTargets} selector=${props.targets}>
      <${WithState} selector=${props.permissions}>
        <${HiddenField} name=${props.name} />
        <${TableContainer} class=${props.class}>
          <${TargetHeadings} noun=${props.noun} />
          <${TargetRows}
            enabling=${props.enabling || false}
            self=${props.self}
          />
        <//>
      <//>
    <//>
  `;
};

export function register() {
  registerElement(ParcelTeams, "parcel-teams", [
    "name",
    "class",
    "enabling",
    "self",
    "targets",
    "permissions",
  ]);
}
